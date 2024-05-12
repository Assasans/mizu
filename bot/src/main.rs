pub mod http;
pub mod dump_performance;
pub mod discord;
pub mod object_storage;
pub mod log;
pub mod halt;

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
use reqwest::Method;

use tokio::{fs, task};
use tokio::process::Command;
use tracing::{debug, error, info, trace};
use tracing_subscriber::{EnvFilter, fmt};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::Shard;
use twilight_http::Client;
use twilight_model::gateway::{Intents, ShardId};
use twilight_model::gateway::event::Event;
use twilight_standby::Standby;
use runtime::cpu::Cpu;
use runtime::exception::Exception;
use runtime::perf_counter::CPU_TIME_LIMIT;
use crate::discord::DiscordInterruptHandler;
use crate::dump_performance::DumpPerformanceHandler;
use crate::halt::HaltHandler;
use crate::http::HttpHandler;
use crate::log::LogHandler;
use crate::object_storage::{ObjectStorage, ObjectStorageHandler};

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

  let object_storage = Arc::new(ObjectStorage::new());
  object_storage.put("amongus", "да я люблю сосать член".as_bytes());

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
    tokio::spawn(handle_event(event, Arc::clone(&http), Arc::clone(&standby), Arc::clone(&object_storage)));
  }

  Ok(())
}

async fn handle_event(
  event: Event,
  http: Arc<Client>,
  standby: Arc<Standby>,
  object_storage: Arc<ObjectStorage>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
  match event {
    Event::MessageCreate(msg) if msg.content.starts_with("!vm") => {
      let code = msg.content.trim_start_matches("!vm").trim_start();
      let code = code.trim_start_matches("```").trim_start_matches("rs\n").trim_end_matches("```").trim();
      let code = format!(r#"#![feature(lang_items)]
#![feature(naked_functions)]

#![no_std]
#![no_main]

mod panic;
mod prelude;

use prelude::*;

{}
"#, code);
      debug!("running code: {}", code);

      let code_filename = "temp/src/main.rs";
      fs::write(code_filename, code).await?;
      let (binary_filename, compile_error, success) = generate_rv_obj().await;
      if !success {
        http.create_message(msg.channel_id).content(&format!("compilation failed: ```c\n{}```", compile_error))?.await?;
        return Ok(());
      }

      let assembly = get_disassembled(&binary_filename).await;
      http.create_message(msg.channel_id).content(&format!(
        "compilation successful: ```x86asm\n{}```",
        if assembly.len() > 1600 { "; too long" } else { &assembly }
      ))?.await?;
      // debug!("{}", assembly);

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
      cpu.ivt.insert(12, Arc::new(Box::new(HttpHandler {
        guild_id: msg.guild_id.unwrap(),
        channel_id: msg.channel_id,
        standby: standby.clone(),
        http: http.clone(),
      })));
      cpu.ivt.insert(13, Arc::new(Box::new(ObjectStorageHandler {
        guild_id: msg.guild_id.unwrap(),
        channel_id: msg.channel_id,
        standby: standby.clone(),
        http: http.clone(),
        object_storage: object_storage.clone()
      })));
      cpu.ivt.insert(14, Arc::new(Box::new(LogHandler {
        guild_id: msg.guild_id.unwrap(),
        channel_id: msg.channel_id,
        standby: standby.clone(),
        http: http.clone(),
      })));
      cpu.ivt.insert(15, Arc::new(Box::new(HaltHandler {})));

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
        if cpu.halt {
          break;
        }

        // if cpu.perf.cpu_time > CPU_TIME_LIMIT {
        //   error!("running too long without yield: {:?} > {:?}", cpu.perf.cpu_time, CPU_TIME_LIMIT);
        //   http.create_message(msg.channel_id).content(&format!("running too long without yield: `{:?} > {:?}`", cpu.perf.cpu_time, CPU_TIME_LIMIT))?.await?;
        //   break;
        // }

        match cpu.check_pending_interrupt() {
          Some(interrupt) => cpu.handle_interrupt(interrupt),
          None => (),
        }
      }

      if cpu.halt {
        http.create_message(msg.channel_id).content(&format!("execution halted: ```c\n// register dump\nperf={:?}\npc = 0x{:x}{}\n{}```", cpu.perf, cpu.pc, cpu.dump_registers(), cpu.csr.dump_csrs()))?.await?;
      } else {
        http.create_message(msg.channel_id).content(&format!("execution finished: ```c\n// register dump\nperf={:?}\npc = 0x{:x}{}\n{}```", cpu.perf, cpu.pc, cpu.dump_registers(), cpu.csr.dump_csrs()))?.await?;
      }
    }
    Event::Ready(_) => {
      info!("shard is ready");
    }
    _ => {}
  };

  Ok(())
}

async fn generate_rv_obj() -> (String, String, bool) {
  let output = Command::new("cargo")
    .current_dir("temp")
    .args(&[
      "+nightly",
      "build"
    ])
    .output()
    .await
    .expect("Failed to generate rv object");
  let stderr = String::from_utf8_lossy(&output.stderr);
  println!("{}", stderr);
  ("target/riscv64g-unknown-mizu-elf/debug/temp".to_owned(), stderr.to_string(), output.status.success())
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
    .args(&["-d", "--visualize-jumps", "-C", obj])
    .output()
    .await
    .expect("Failed to disassemble rv binary");
  return String::from_utf8_lossy(&output.stdout).to_string();
}
