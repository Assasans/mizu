pub mod http;
pub mod dump_performance;
pub mod discord;
pub mod object_storage;
pub mod log;
pub mod halt;
pub mod time;
pub mod sipi;
mod execution_context;

use std::{env, mem};
use std::collections::HashMap;
use std::error::Error;
use std::ffi::{c_char, CString};
use std::fmt::Write;
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
use twilight_model::http::attachment::Attachment;
use twilight_standby::Standby;
use regex::{Captures, Regex};
use thiserror::Error;
use tokio::sync::{Mutex, RwLock};
use twilight_model::channel::Message;
use twilight_model::channel::message::ReactionType;
use twilight_model::id::Id;
use twilight_model::id::marker::GuildMarker;
use hal_types::discord::{action, discord_create_reaction_t, discord_event_add_reaction_t, discord_message_t};
use hal_types::StringPtr;
use runtime::apic::INTERRUPT_PRIORITY_NORMAL;
use runtime::bus::{Bus, BusMemoryExt};
use runtime::cpu::Cpu;
use runtime::csr;
use runtime::csr::MCAUSE;
use runtime::exception::Exception;
use runtime::interrupt::Interrupt;
use runtime::isolate::Isolate;
use runtime::param::HARDWARE_BASE;
use runtime::perf_counter::CPU_TIME_LIMIT;
use crate::discord::{DiscordInterruptHandler, MemoryObject};
use crate::dump_performance::DumpPerformanceHandler;
use crate::execution_context::ExecutionContext;
use crate::halt::HaltHandler;
use crate::http::HttpHandler;
use crate::log::LogHandler;
use crate::object_storage::{ObjectStorage, ObjectStorageHandler};
use crate::sipi::SipiHandler;
use crate::time::TimeHandler;

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
  let intents = Intents::GUILD_MESSAGES | Intents::DIRECT_MESSAGES | Intents::MESSAGE_CONTENT | Intents::GUILD_MESSAGE_REACTIONS;

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

  let contexts = Arc::new(Contexts::new());

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
    tokio::spawn(handle_event(event, Arc::clone(&http), Arc::clone(&standby), Arc::clone(&object_storage), Arc::clone(&contexts)));
  }

  Ok(())
}

pub struct Contexts {
  pub contexts: RwLock<HashMap<Id<GuildMarker>, Arc<Mutex<ExecutionContext>>>>,
}

impl Contexts {
  pub fn new() -> Self {
    Contexts {
      contexts: RwLock::new(HashMap::new())
    }
  }
}

pub const ENABLE_DISCORD_INTERRUPTS: bool = true;

async fn handle_event(
  event: Event,
  http: Arc<Client>,
  standby: Arc<Standby>,
  object_storage: Arc<ObjectStorage>,
  contexts: Arc<Contexts>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
  match event {
    Event::MessageCreate(msg) if msg.content.starts_with("!vm") => {
      let code = msg.content.trim_start_matches("!vm").trim_start();
      let code = code.trim_start_matches("```").trim_start_matches("rs\n").trim_end_matches("```").trim();
      let code = format!(r#"#![feature(lang_items)]
#![feature(naked_functions)]

#![no_std]
#![no_main]

mod prelude;

use prelude::*;

{}
"#, code);
      debug!("running code: {}", code);

      let context = {
        let mut contexts = contexts.contexts.write().await;
        let context = contexts.entry(msg.guild_id.unwrap());
        context.or_insert_with(|| Arc::new(Mutex::new(ExecutionContext::new()))).clone()
      };

      let code = compile(&code, &msg, &http).await?;
      let bus = Arc::new(Bus::new(code));
      let isolate = {
        let mut context = context.lock().await;
        context.http = Some(http.clone());
        context.channel_id = Some(msg.channel_id);

        context.isolate.insert(Isolate::new(bus)).clone()
      };

      {
        let cpu = isolate.get_bootstrap_core();
        let mut cpu = cpu.lock().await;
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
          object_storage: object_storage.clone(),
        })));
        cpu.ivt.insert(14, Arc::new(Box::new(LogHandler {
          guild_id: msg.guild_id.unwrap(),
          channel_id: msg.channel_id,
          standby: standby.clone(),
          http: http.clone(),
        })));
        cpu.ivt.insert(15, Arc::new(Box::new(HaltHandler {})));
        cpu.ivt.insert(16, Arc::new(Box::new(TimeHandler {})));
        cpu.ivt.insert(17, Arc::new(Box::new(SipiHandler {})));
      }

      let wfi = {
        let cpu = isolate.get_bootstrap_core();
        let mut cpu = cpu.lock().await;
        cpu.wfi.clone()
      };
      'wfi: loop {
        wfi.wait_for(|wfi| *wfi == false).await;
        http.create_message(msg.channel_id).content(&format!("wfi: reset"))?.await?;

        let cpu = isolate.get_bootstrap_core();
        let mut cpu = cpu.lock().await;
        loop {
          match cpu.run_tick().await? {
            TickResult::Continue => continue,
            TickResult::Exception(exception) => {
              http.create_message(msg.channel_id).content(&format!("cpu exception: {:?}", exception))?.await?;
            }
            TickResult::Eof => {
              http.create_message(msg.channel_id).content(&format!("execution finished: ```c\n// register dump\nperf={:?}\npc = 0x{:x}{}\n{}```", cpu.perf, cpu.pc, cpu.dump_registers(), cpu.csr.dump_csrs()))?.await?;
            }
            TickResult::Halt => {
              http.create_message(msg.channel_id).content(&format!("execution halted: ```c\n// register dump\nperf={:?}\npc = 0x{:x}{}\n{}```", cpu.perf, cpu.pc, cpu.dump_registers(), cpu.csr.dump_csrs()))?.await?;
            }
            TickResult::TimeLimit => {
              http.create_message(msg.channel_id).content(&format!("running too long without yield: `{:?} > {:?}`", cpu.perf.cpu_time, CPU_TIME_LIMIT))?.await?;
            }
            TickResult::WaitForInterrupt => {
              http.create_message(msg.channel_id).content(&format!("wfi: waiting for interrupt at `{:#08x}`", cpu.pc))?.await?;
              continue 'wfi;
            }
          }
          break 'wfi;
        }
      }
    }
    Event::MessageCreate(msg) => {
      debug!("create message: {:?}", msg.id);
      if msg.author.bot || msg.content.len() > 200 {
        return Ok(());
      }

      if !ENABLE_DISCORD_INTERRUPTS {
        return Ok(());
      }

      dispatch_interrupt(&contexts, msg.guild_id.unwrap(), |cpu| {
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
          object_storage: object_storage.clone(),
        })));
        cpu.ivt.insert(14, Arc::new(Box::new(LogHandler {
          guild_id: msg.guild_id.unwrap(),
          channel_id: msg.channel_id,
          standby: standby.clone(),
          http: http.clone(),
        })));
        cpu.ivt.insert(15, Arc::new(Box::new(HaltHandler {})));

        let event = discord_message_t {
          id: msg.id.get(),
          channel_id: msg.channel_id.get(),
          author_id: msg.author.id.get(),
          content: StringPtr((HARDWARE_BASE + 0x19000) as *const c_char),
        };
        cpu.bus.write_string(event.content.0 as u64, &msg.content).unwrap();

        (action::EVENT_MESSAGE_CREATE, event)
      }).await;
    }
    Event::ReactionAdd(reaction) => {
      debug!("add reaction: {:?}", reaction);

      if !ENABLE_DISCORD_INTERRUPTS {
        return Ok(());
      }

      dispatch_interrupt(&contexts, reaction.guild_id.unwrap(), |cpu| {
        cpu.ivt.insert(10, Arc::new(Box::new(DiscordInterruptHandler {
          guild_id: reaction.guild_id.unwrap(),
          channel_id: reaction.channel_id,
          standby: standby.clone(),
          http: http.clone(),
        })));
        cpu.ivt.insert(11, Arc::new(Box::new(DumpPerformanceHandler {
          guild_id: reaction.guild_id.unwrap(),
          channel_id: reaction.channel_id,
          standby: standby.clone(),
          http: http.clone(),
        })));
        cpu.ivt.insert(12, Arc::new(Box::new(HttpHandler {
          guild_id: reaction.guild_id.unwrap(),
          channel_id: reaction.channel_id,
          standby: standby.clone(),
          http: http.clone(),
        })));
        cpu.ivt.insert(13, Arc::new(Box::new(ObjectStorageHandler {
          guild_id: reaction.guild_id.unwrap(),
          channel_id: reaction.channel_id,
          standby: standby.clone(),
          http: http.clone(),
          object_storage: object_storage.clone(),
        })));
        cpu.ivt.insert(14, Arc::new(Box::new(LogHandler {
          guild_id: reaction.guild_id.unwrap(),
          channel_id: reaction.channel_id,
          standby: standby.clone(),
          http: http.clone(),
        })));
        cpu.ivt.insert(15, Arc::new(Box::new(HaltHandler {})));

        if let ReactionType::Unicode { name } = &reaction.emoji {
          let event = discord_event_add_reaction_t {
            channel_id: reaction.channel_id.get(),
            message_id: reaction.message_id.get(),
            user_id: reaction.user_id.get(),
            emoji: StringPtr((HARDWARE_BASE + 0x19000) as *const c_char),
          };
          cpu.bus.write_string(event.emoji.0 as u64, name).unwrap();

          (action::EVENT_REACTION_ADD, event)
        } else {
          todo!()
        }
      }).await;
    }
    Event::Ready(_) => {
      info!("shard is ready");
    }
    _ => {}
  };

  Ok(())
}

#[derive(Error, Debug)]
pub enum CompileError {
  #[error("compilation failed")]
  Error
}

async fn compile(code: &str, msg: &Message, http: &Client) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
  let code_filename = "temp/src/main.rs";
  fs::write(code_filename, code).await?;
  let (binary_filename, compile_error, success) = generate_rv_obj().await;
  if !success {
    let attachments = if compile_error.len() > 1600 {
      vec![Attachment::from_bytes("error.log".to_owned(), compile_error.as_bytes().to_owned(), 1)]
    } else {
      vec![]
    };

    if compile_error.len() > 1800 {
      http.create_message(msg.channel_id)
        .content("compilation failed").unwrap()
        .attachments(&attachments)?.await?;
    } else {
      http.create_message(msg.channel_id).content(&format!("compilation failed: ```c\n{}```", compile_error))?.await?;
    }
    return Err(Box::new(CompileError::Error));
  }

  let assembly = get_disassembled(&binary_filename).await;
  let assembly = Regex::new(r"(?m)^ffffffff80[0-9a-f]{6}").unwrap().replace_all(&assembly, |captures: &Captures| {
    let address = u64::from_str_radix(captures.get(0).unwrap().as_str(), 16).unwrap();
    let base_address = 0xffffffff80000000u64;
    format!("${:04x}", address - base_address)
  });
  // http.create_message(msg.channel_id).content(&format!(
  //   "compilation successful: ```mips\n{}```",
  //   if assembly.len() > 1800 { "; too long" } else { &assembly }
  // ))?.await?;
  // debug!("{}", assembly);

  generate_rv_binary(&binary_filename).await;
  Ok(fs::read(format!("{}.bin", binary_filename)).await?)
}

async fn dispatch_interrupt<T>(contexts: &Arc<Contexts>, guild_id: Id<GuildMarker>, block: impl FnOnce(&mut Cpu) -> (u64, T)) {
  info!("dispatch int");
  let context = {
    let mut contexts = contexts.contexts.write().await;
    let context = contexts.entry(guild_id);
    context.or_insert_with(|| Arc::new(Mutex::new(ExecutionContext::new()))).clone()
  };
  let context = context.lock().await;
  let http = context.http.as_ref().unwrap().clone();
  let channel_id = context.channel_id.as_ref().unwrap().clone();
  let cpu = context.isolate.as_ref().unwrap().get_bootstrap_core();
  let mut cpu = cpu.lock().await;

  cpu.halt = false;

  let (id, event) = block(&mut cpu);
  cpu.bus.write_struct(HARDWARE_BASE + 0x16000, &event).unwrap();
  cpu.regs[10] = id;
  cpu.regs[11] = HARDWARE_BASE + 0x16000;

  info!("dispatching interrupt");
  cpu.apic.dispatch(Interrupt::PlatformDefined17, INTERRUPT_PRIORITY_NORMAL);
  info!("resetting wfi");
  cpu.wfi.set(false);

  let isolate = cpu.isolate.as_ref().unwrap().upgrade().unwrap();
  isolate.wake();
}

#[async_trait]
pub trait CpuExt {
  async fn run_tick(&mut self) -> Result<TickResult, Box<dyn Error + Send + Sync>>;
}

pub enum TickResult {
  Continue,
  Exception(Exception),
  Eof,
  Halt,
  TimeLimit,
  WaitForInterrupt
}

#[async_trait]
impl CpuExt for Cpu {
  async fn run_tick(&mut self) -> Result<TickResult, Box<dyn Error + Send + Sync>> {
    if self.wfi.get() {
      return Ok(TickResult::WaitForInterrupt);
    }

    let inst = match self.fetch() {
      Ok(inst) => inst,
      Err(exception) => {
        self.handle_exception(exception);
        if let Exception::InstructionAccessFault(0) = &exception {
          return Ok(TickResult::Eof);
        }
        if exception.is_fatal() {
          error!("fetch failed: {:?}", exception);
          return Ok(TickResult::Exception(exception));
        }

        return Ok(TickResult::Exception(exception));
      }
    };

    match self.execute(inst).await {
      Ok(new_pc) => self.pc = new_pc,
      Err(exception) => {
        self.handle_exception(exception);
        if exception.is_fatal() {
          error!("execute failed: {:?}", exception);
          return Ok(TickResult::Exception(exception));
        }

        return Ok(TickResult::Exception(exception));
      }
    };

    self.perf.instructions_retired += 1;
    if self.halt {
      return Ok(TickResult::Halt);
    }

    if self.csr.load(csr::machine::POWERSTATE) == 1 && self.perf.cpu_time > CPU_TIME_LIMIT {
      error!("running too long without yield: {:?} > {:?}", self.perf.cpu_time, CPU_TIME_LIMIT);
      return Ok(TickResult::TimeLimit);
    }

    // if self.csr.load(MCAUSE) == 0 {
    //   error!("exited from trap");
    //   self.saved_regs.fill(0);
    //   return Ok(TickResult::Continue); // Exited from trap
    // }

    match self.check_pending_interrupt() {
      Some(interrupt) => self.handle_interrupt(interrupt),
      None => (),
    }

    Ok(TickResult::Continue)
  }
}

async fn generate_rv_obj() -> (String, String, bool) {
  let output = Command::new("cargo")
    .env("CC", "/usr/bin/clang")
    .env("CXX", "/usr/bin/clang++")
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
    .args(&[
      "--disassemble=_start",
      "--no-show-raw-insn",
      // "--visualize-jumps",
      // "--source",
      "-C",
      obj
    ])
    .output()
    .await
    .expect("Failed to disassemble rv binary");
  return String::from_utf8_lossy(&output.stdout).to_string();
}
