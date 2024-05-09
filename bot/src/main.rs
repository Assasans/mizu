use std::{env, mem};
use std::error::Error;
use std::ffi::{c_char, CString};
use std::marker::PhantomData;
use std::mem::size_of;
use std::num::NonZeroU64;
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
use twilight_http::request::channel::reaction::RequestReactionType;
use twilight_model::channel::Channel;
use twilight_model::channel::message::{MessageFlags, ReactionType};
use twilight_model::gateway::{Intents, ShardId};
use twilight_model::gateway::event::Event;
use twilight_model::gateway::payload::incoming::MessageCreate;
use twilight_model::id::Id;
use twilight_model::id::marker::{ChannelMarker, GuildMarker, StickerMarker};
use twilight_standby::Standby;
use runtime::bus::{Bus, BusMemoryExt};
use runtime::cpu::{Cpu, InterruptHandler};
use runtime::exception::Exception;
use runtime::param::DRAM_BASE;
use runtime::perf_counter::CPU_TIME_LIMIT;

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

  let standby = Arc::new(Standby::new());

  // Startup the event loop to process each event in the event stream as they
  // come in.
  while let item = shard.next_event().await {
    let Ok(event) = item else {
      tracing::warn!(source = ?item.unwrap_err(), "error receiving event");

      continue;
    };
    // Update the cache.
    cache.update(&event);
    standby.process(&event);

    // Spawn a new task to handle the event
    tokio::spawn(handle_event(event, Arc::clone(&http), Arc::clone(&standby)));
  }

  Ok(())
}

async fn handle_event(
  event: Event,
  http: Arc<Client>,
  standby: Arc<Standby>,
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
      http.create_message(msg.channel_id).content(&format!(
        "compilation successful: ```x86asm\n{}```",
        if assembly.len() > 1600 { "; too long" } else { &assembly }
      ))?.await?;
      debug!("{}", assembly);

      generate_rv_binary(&binary_filename).await;
      let code = fs::read(format!("{}.bin", binary_filename)).await?;
      let mut cpu = Cpu::new(code);
      cpu.ivt.insert(10, Arc::new(Box::new(DiscordInterruptHandler {
        guild_id: msg.guild_id.unwrap(),
        channel_id: msg.channel_id,
        standby: standby.clone(),
        http: http.clone(),
      })));
      cpu.ivt.insert(11, Arc::new(Box::new(DumpPerformanceHandler {
        guild_id: msg.guild_id.unwrap(),
        channel_id: msg.channel_id,
        standby: standby.clone(),
        http: http.clone(),
      })));

      loop {
        let inst = match cpu.fetch() {
          Ok(inst) => inst,
          Err(exception) => {
            cpu.handle_exception(exception);
            if let Exception::InstructionAccessFault(0) = &exception {
              break;
            }
            if exception.is_fatal() {
              http.create_message(msg.channel_id).content(&format!("fetch failed: {:?}", exception))?.await?;
              error!("fetch failed: {:?}", exception);
              break;
            }

            break;
          }
        };

        match cpu.execute(inst).await {
          Ok(new_pc) => cpu.pc = new_pc,
          Err(exception) => {
            cpu.handle_exception(exception);
            if exception.is_fatal() {
              http.create_message(msg.channel_id).content(&format!("execute failed: {:?}", exception))?.await?;
              error!("execute failed: {:?}", exception);
              break;
            }
            break;
          }
        };
        cpu.perf.instructions_retired += 1;

        if cpu.perf.cpu_time > CPU_TIME_LIMIT {
          error!("running too long without yield: {:?} > {:?}", cpu.perf.cpu_time, CPU_TIME_LIMIT);
          http.create_message(msg.channel_id).content(&format!("running too long without yield: `{:?} > {:?}`", cpu.perf.cpu_time, CPU_TIME_LIMIT))?.await?;
          break;
        }

        match cpu.check_pending_interrupt() {
          Some(interrupt) => cpu.handle_interrupt(interrupt),
          None => (),
        }
      }

      http.create_message(msg.channel_id).content(&format!("execution finished: ```c\n// register dump\nperf={:?}\npc = 0x{:x}{}```", cpu.perf, cpu.pc, cpu.dump_registers()))?.await?;
    }
    Event::Ready(_) => {
      info!("shard is ready");
    }
    _ => {}
  };

  Ok(())
}

struct DiscordInterruptHandler {
  guild_id: Id<GuildMarker>,
  channel_id: Id<ChannelMarker>,
  standby: Arc<Standby>,
  http: Arc<Client>,
}

#[repr(C)]
#[derive(Debug)]
pub struct discord_create_message_t {
  pub channel_id: u64,
  pub flags: u64,
  pub reply: Option<NonZeroU64>,
  pub stickers: [Option<NonZeroU64>; 3],
  pub content: *const c_char,
}

unsafe impl Send for discord_create_message_t {}

#[repr(C)]
#[derive(Debug)]
pub struct discord_create_reaction_t {
  pub channel_id: u64,
  pub message_id: u64,
  pub emoji: *const c_char,
}

unsafe impl Send for discord_create_reaction_t {}

#[repr(C)]
#[derive(Debug)]
pub struct discord_message_t {
  pub id: u64,
  pub channel_id: u64,
  pub author_id: u64,
  pub content: *const c_char,
}

unsafe impl Send for discord_message_t {}

#[async_trait]
impl InterruptHandler for DiscordInterruptHandler {
  async fn handle(&self, cpu: &mut Cpu) {
    let id = cpu.regs[10];
    let address = cpu.regs[11];
    debug!("discord call: id={} address=0x{:x}", id, address);

    match id {
      1 => {
        let request = cpu.bus.read_struct::<discord_create_message_t>(address).unwrap();
        debug!("request: {:?}", request);

        let mut builder = self.http.create_message(Id::new(request.channel_id));

        let content = if !request.content.is_null() {
          Some(cpu.bus.read_string(request.content as u64).unwrap().to_string_lossy().to_string())
        } else {
          None
        };
        debug!("content: {:?}", content);
        if let Some(content) = &content {
          builder = builder.content(&content).unwrap();
        }

        builder = builder.flags(MessageFlags::from_bits(request.flags).unwrap());

        let stickers = request.stickers.iter()
          .filter_map(|it| *it)
          .map(|it| Id::<StickerMarker>::from(it))
          .collect::<Vec<_>>();
        builder = builder.sticker_ids(&stickers).unwrap();

        if let Some(reply) = request.reply {
          builder = builder.reply(Id::from(reply));
        }

        let response = builder
          .await.unwrap()
          .model().await.unwrap();
        cpu.regs[10] = response.id.get();
      }
      2 => {
        let request = cpu.bus.read_struct::<discord_create_reaction_t>(address).unwrap();
        debug!("request: {:?}", request);

        self.http.create_reaction(
          Id::new(request.channel_id),
          Id::new(request.message_id),
          &RequestReactionType::Unicode { name: cpu.bus.read_string(request.emoji as u64).unwrap().to_str().unwrap() },
        ).await.unwrap();
        cpu.regs[10] = 0;
      }
      10 => {
        let message = self.standby.wait_for(self.guild_id, |event: &Event| {
          if let Event::MessageCreate(message) = event {
            !message.author.bot
          } else {
            false
          }
        }).await.unwrap();
        let message = if let Event::MessageCreate(message) = message {
          message
        } else {
          unreachable!()
        };
        debug!("got message: {:?}", message);

        let ffi_message = discord_message_t {
          id: message.id.get(),
          channel_id: message.channel_id.get(),
          author_id: message.author.id.get(),
          content: (DRAM_BASE + 0x9900) as *const c_char,
        };

        cpu.bus.write_string(ffi_message.content as u64, &message.content).unwrap();
        cpu.bus.write_struct(DRAM_BASE + 0x6000, &ffi_message).unwrap();
        cpu.regs[10] = DRAM_BASE + 0x6000;
      }
      _ => unimplemented!()
    }
  }
}

struct DumpPerformanceHandler {
  guild_id: Id<GuildMarker>,
  channel_id: Id<ChannelMarker>,
  standby: Arc<Standby>,
  http: Arc<Client>,
}

#[async_trait]
impl InterruptHandler for DumpPerformanceHandler {
  async fn handle(&self, cpu: &mut Cpu) {
    self.http.create_message(self.channel_id)
      .content(&format!("performance dump: ```c\nperf={:?}\npc = 0x{:x}{}```", cpu.perf, cpu.pc, cpu.dump_registers())).unwrap()
      .await.unwrap();
    cpu.perf.reset();
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
