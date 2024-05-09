use std::{env, mem};
use std::error::Error;
use std::ffi::{c_char, CString};
use std::mem::size_of;
use std::os::raw::c_uchar;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;

use tokio::{fs, task};
use tokio::process::Command;
use tracing::{debug, error, info, trace};
use tracing_subscriber::{EnvFilter, fmt};
use tracing_subscriber::fmt::format;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::{EventTypeFlags, Shard};
use twilight_http::Client;
use twilight_model::channel::Channel;
use twilight_model::channel::message::MessageFlags;
use twilight_model::gateway::{Intents, ShardId};
use twilight_model::gateway::event::Event;
use twilight_model::id::Id;
use twilight_model::id::marker::ChannelMarker;
use runtime::bus::{Bus, BusMemoryExt};
use runtime::cpu::{Cpu, InterruptHandler};
use runtime::exception::Exception;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
  tracing_subscriber::registry()
    .with(fmt::layer())
    .with(EnvFilter::from_default_env())
    .init();

  let token = env::var("DISCORD_TOKEN")?;
  info!("starting...");

  // Specify intents requesting events about things like new and updated
  // messages in a guild and direct messages.
  let intents = Intents::GUILD_MESSAGES | Intents::DIRECT_MESSAGES | Intents::MESSAGE_CONTENT;

  // Create a single shard.
  let mut shard = Shard::new(ShardId::ONE, token.clone(), intents);

  // The http client is separate from the gateway, so startup a new
  // one, also use Arc such that it can be cloned to other threads.
  let http = Arc::new(Client::new(token));

  // Since we only care about messages, make the cache only process messages.
  let cache = InMemoryCache::builder()
    .resource_types(ResourceType::MESSAGE)
    .build();

  // Startup the event loop to process each event in the event stream as they
  // come in.
  while let item = shard.next_event().await {
    let Ok(event) = item else {
      tracing::warn!(source = ?item.unwrap_err(), "error receiving event");

      continue;
    };
    // Update the cache.
    cache.update(&event);

    // Spawn a new task to handle the event
    tokio::spawn(handle_event(event, Arc::clone(&http)));
  }

  Ok(())
}

async fn handle_event(
  event: Event,
  http: Arc<Client>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
  match event {
    Event::MessageCreate(msg) if msg.content.starts_with("!vm") => {
      let code = msg.content.trim_start_matches("!vm").trim_start();
      let code = code.trim_start_matches("```").trim_start_matches("c\n").trim_end_matches("```").trim();
      debug!("running code: {}", code);

      let code_filename = "bot/temp/main.c";
      fs::write(code_filename, code).await?;
      let (binary_filename, compile_error) = generate_rv_obj(&code_filename).await;
      if !compile_error.is_empty() {
        http.create_message(msg.channel_id).content(&format!("compilation failed: ```c\n{}```", compile_error))?.await?;
        return Ok(());
      }

      let assembly = get_disassembled(&binary_filename).await;
      http.create_message(msg.channel_id).content(&format!("compilation successful: ```x86asm\n{}```", assembly))?.await?;

      generate_rv_binary(&binary_filename).await;
      let code = fs::read(format!("{}.bin", binary_filename)).await?;
      let mut cpu = Cpu::new(code);
      cpu.ivt.insert(10, Box::new(DiscordInterruptHandler {
        channel_id: msg.channel_id,
        http: http.clone(),
      }));

      loop {
        let inst = match cpu.fetch() {
          Ok(inst) => inst,
          Err(exception) => {
            if let Exception::LoadAccessFault(0) = &exception {
              break;
            }

            http.create_message(msg.channel_id).content(&format!("fetch failed: {:?}", exception))?.await?;
            error!("fetch failed: {:?}", exception);
            break;
          }
        };

        match cpu.execute(inst).await {
          Ok(new_pc) => cpu.pc = new_pc,
          Err(exception) => {
            http.create_message(msg.channel_id).content(&format!("execute failed: {:?}", exception))?.await?;
            error!("execute failed: {:?}", exception);
            break;
          }
        };
      }

      http.create_message(msg.channel_id).content(&format!("execution finished: ```c\n// register dump\npc = 0x{:x}\n{}```", cpu.pc, cpu.dump_registers()))?.await?;
    }
    Event::Ready(_) => {
      info!("shard is ready");
    }
    _ => {}
  };

  Ok(())
}

struct DiscordInterruptHandler {
  channel_id: Id<ChannelMarker>,
  http: Arc<Client>,
}

#[repr(C)]
#[derive(Debug)]
pub struct discord_create_message_t {
  pub flags: u64,
  pub reply: u64,
  pub stickers: [u64; 1],
  pub content: *const c_char,
}

unsafe impl Send for discord_create_message_t {}

#[async_trait]
impl InterruptHandler for DiscordInterruptHandler {
  async fn handle(&self, regs: &mut [u64; 32], bus: &mut Bus) {
    let id = regs[10];
    let address = regs[11];
    debug!("discord call: id={} address=0x{:x}", id, address);

    match id {
      1 => {
        let request = bus.read_struct::<discord_create_message_t>(address).unwrap();
        debug!("request: {:?}", request);

        let content = bus.read_string(request.content as u64).unwrap().to_string_lossy().to_string();
        debug!("content: {}", content);

        let response = self.http.create_message(self.channel_id)
          .content(&content).unwrap()
          .flags(MessageFlags::from_bits(request.flags).unwrap())
          .sticker_ids(&request.stickers.map(|id| Id::new(id))).unwrap()
          .reply(Id::new(request.reply))
          .await.unwrap()
          .model().await.unwrap();
        regs[10] = response.id.get();
      }
      _ => unimplemented!()
    }
  }
}

async fn generate_rv_obj(input: &str) -> (String, String) {
  let hal_root = PathBuf::from(env::var("HAL_ROOT").unwrap());
  let cc = "clang";
  let pieces: Vec<&str> = input.split(".").collect();
  let output = Command::new(cc)
    .args(&[
      "-O1",
      "-nostdlib",
      "-march=rv64g",
      "--target=riscv64",
      "-mabi=lp64",
      "-mno-relax",
      &format!("-I{}", hal_root.to_str().unwrap()),
      &format!("-Wl,-T{}", hal_root.join("memmap.ld").to_str().unwrap()),
      "-o", &pieces[0],
      input
    ])
    .output()
    .await
    .expect("Failed to generate rv object");
  let stderr = String::from_utf8_lossy(&output.stderr);
  println!("{}", stderr);
  (pieces[0].to_owned(), stderr.to_string())
}

async fn generate_rv_binary(obj: &str) {
  let objcopy = "llvm-objcopy";
  let output = Command::new(objcopy)
    .args(&["-O", "binary", obj, &format!("{}.bin", obj.to_owned())])
    .output()
    .await
    .expect("Failed to generate rv binary");
  println!("{}", String::from_utf8_lossy(&output.stderr));
}

async fn get_disassembled(obj: &str) -> String {
  let objcopy = "riscv64-unknown-elf-objdump";
  let output = Command::new(objcopy)
    .args(&["-d", obj])
    .output()
    .await
    .expect("Failed to disassemble rv binary");
  return String::from_utf8_lossy(&output.stdout).to_string();
}
