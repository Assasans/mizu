mod environment;
mod execution_context;

use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::sync::Arc;

use async_trait::async_trait;
use mizu_hal_discord::discord::discord_ex_event::DiscordExEventUnion;
use mizu_hal_discord::discord::{DiscordExEvent, Emoji, IncomingMessage, ReactionCreate};
use mizu_hal_discord::prost::Message as ProstMessage;
use mizu_hal_types::syscall;
use regex::{Captures, Regex};
use runtime::apic::INTERRUPT_PRIORITY_NORMAL;
use runtime::bus::{Bus, BusMemoryExt};
use runtime::cpu::Cpu;
use runtime::csr;
use runtime::exception::Exception;
use runtime::interrupt::Interrupt;
use runtime::isolate::Isolate;
use runtime::memory::HARDWARE_BASE;
use runtime::perf_counter::CPU_TIME_LIMIT;
use thiserror::Error;
use tokio::fs;
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::Shard;
use twilight_http::Client;
use twilight_model::channel::message::ReactionType;
use twilight_model::channel::Message;
use twilight_model::gateway::event::Event;
use twilight_model::gateway::{Intents, ShardId};
use twilight_model::http::attachment::Attachment;
use twilight_model::id::marker::GuildMarker;
use twilight_model::id::Id;
use twilight_standby::Standby;

use crate::environment::discord::DiscordInterruptHandler;
use crate::environment::discord_ex::DiscordExInterruptHandler;
use crate::environment::dump_performance::DumpPerformanceHandler;
use crate::environment::halt::HaltHandler;
use crate::environment::http::HttpHandler;
use crate::environment::interrupt::IntHandler;
use crate::environment::log::LogHandler;
use crate::environment::object_storage::{ObjectStorage, ObjectStorageHandler};
use crate::environment::png::PngHandler;
use crate::environment::sipi::SipiHandler;
use crate::environment::time::TimeHandler;
use crate::execution_context::ExecutionContext;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
  tracing_subscriber::registry().with(fmt::layer()).with(EnvFilter::from_default_env()).init();

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
  let cache = InMemoryCache::builder().resource_types(ResourceType::MESSAGE).build();

  let standby = Arc::new(Standby::new());

  let object_storage = Arc::new(ObjectStorage::new());
  object_storage.put("amongus", "да я люблю сосать член".as_bytes());

  let contexts = Arc::new(Contexts::new());

  // Startup the event loop to process each event in the event stream as they
  // come in.
  loop {
    let event = match shard.next_event().await {
      Ok(event) => event,
      Err(error) => {
        warn!(source = ?error, "error receiving event");
        continue;
      }
    };

    // Update the cache.
    cache.update(&event);
    standby.process(&event);

    // Spawn a new task to handle the event
    tokio::spawn(handle_event(
      event,
      Arc::clone(&http),
      Arc::clone(&standby),
      Arc::clone(&object_storage),
      Arc::clone(&contexts),
    ));
  }
}

pub struct Contexts {
  pub contexts: RwLock<HashMap<Id<GuildMarker>, Arc<ExecutionContext>>>,
}

impl Default for Contexts {
  fn default() -> Self {
    Self::new()
  }
}

impl Contexts {
  pub fn new() -> Self {
    Self {
      contexts: RwLock::new(HashMap::new()),
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
      let code = format!(
        r#"#![feature(lang_items)]
#![feature(naked_functions)]

#![no_std]
#![no_main]

mod prelude;

use prelude::*;

{}
"#,
        code
      );
      debug!("running code: {}", code);

      let context = {
        let mut contexts = contexts.contexts.write().await;
        let context = contexts.entry(msg.guild_id.unwrap());
        context.or_insert_with(|| Arc::new(ExecutionContext::new())).clone()
      };
      *context.http.lock().await = Some(http.clone());
      *context.channel_id.lock().await = Some(msg.channel_id);

      let code = compile(&code, &msg, &http).await?;
      let bus = Arc::new(Bus::new(code));
      let isolate = context.isolate.lock().await.insert(Isolate::new(bus)).clone();

      // Initialize environment
      {
        let cpu = isolate.get_bootstrap_core();
        let mut cpu = cpu.lock().await;
        cpu.ivt.insert(
          syscall::SYSCALL_DISCORD,
          Arc::new(Box::new(DiscordInterruptHandler {
            context: context.clone(),
            guild_id: msg.guild_id.unwrap(),
            standby: standby.clone(),
          })),
        );
        cpu.ivt.insert(
          syscall::SYSCALL_DISCORD_EX,
          Arc::new(Box::new(DiscordExInterruptHandler {
            context: context.clone(),
            guild_id: msg.guild_id.unwrap(),
            standby: standby.clone(),
          })),
        );
        cpu.ivt.insert(
          syscall::SYSCALL_PERF_DUMP,
          Arc::new(Box::new(DumpPerformanceHandler { context: context.clone() })),
        );
        cpu
          .ivt
          .insert(syscall::SYSCALL_HTTP, Arc::new(Box::new(HttpHandler { context: context.clone() })));
        cpu.ivt.insert(
          syscall::SYSCALL_OBJECT_STORAGE,
          Arc::new(Box::new(ObjectStorageHandler {
            context: context.clone(),
            object_storage: object_storage.clone(),
          })),
        );
        cpu
          .ivt
          .insert(syscall::SYSCALL_LOG, Arc::new(Box::new(LogHandler { context: context.clone() })));
        cpu.ivt.insert(syscall::SYSCALL_HALT, Arc::new(Box::new(HaltHandler {})));
        cpu.ivt.insert(syscall::SYSCALL_TIME, Arc::new(Box::new(TimeHandler {})));
        cpu
          .ivt
          .insert(syscall::SYSCALL_SIPI, Arc::new(Box::new(SipiHandler { context: context.clone() })));
        cpu
          .ivt
          .insert(syscall::SYSCALL_INT, Arc::new(Box::new(IntHandler { context: context.clone() })));
        cpu
          .ivt
          .insert(syscall::SYSCALL_PNG, Arc::new(Box::new(PngHandler {})));
      }

      context.run_core(isolate.get_bootstrap_core(), None).await?;
    }
    Event::MessageCreate(msg) => {
      debug!("create message: {:?}", msg.id);
      if msg.author.bot || msg.content.len() > 200 {
        return Ok(());
      }

      if !ENABLE_DISCORD_INTERRUPTS {
        return Ok(());
      }

      dispatch_interrupt(&contexts, msg.guild_id.unwrap(), |context, cpu| {
        cpu.ivt.insert(
          syscall::SYSCALL_DISCORD,
          Arc::new(Box::new(DiscordInterruptHandler {
            context: context.clone(),
            guild_id: msg.guild_id.unwrap(),
            standby: standby.clone(),
          })),
        );
        cpu.ivt.insert(
          syscall::SYSCALL_DISCORD_EX,
          Arc::new(Box::new(DiscordExInterruptHandler {
            context,
            guild_id: msg.guild_id.unwrap(),
            standby: standby.clone(),
          })),
        );

        let data = IncomingMessage {
          id: msg.id.get(),
          channel_id: 0,
          guild_id: None,
          author: None,
          content: msg.content.to_owned(),
          attachments: vec![],
          embeds: vec![],
          timestamp: "".to_string(),
          edited_timestamp: "".to_string(),
          tts: false,
          webhook_id: None,
          mentions: vec![],
          mention_everyone: false,
          mentioned_roles: vec![],
          r#type: 0,
        };
        let event = DiscordExEvent {
          discord_ex_event_union: Some(DiscordExEventUnion::MessageCreate(data)),
        };
        event.encode_to_vec()
      })
      .await;
    }
    Event::ReactionAdd(reaction) => {
      debug!("add reaction: {:?}", reaction);

      if !ENABLE_DISCORD_INTERRUPTS {
        return Ok(());
      }

      dispatch_interrupt(&contexts, reaction.guild_id.unwrap(), |context, cpu| {
        cpu.ivt.insert(
          syscall::SYSCALL_DISCORD,
          Arc::new(Box::new(DiscordInterruptHandler {
            context: context.clone(),
            guild_id: reaction.guild_id.unwrap(),
            standby: standby.clone(),
          })),
        );
        cpu.ivt.insert(
          syscall::SYSCALL_DISCORD_EX,
          Arc::new(Box::new(DiscordExInterruptHandler {
            context,
            guild_id: reaction.guild_id.unwrap(),
            standby: standby.clone(),
          })),
        );

        let data = ReactionCreate {
          user_id: reaction.user_id.get(),
          channel_id: reaction.channel_id.get(),
          message_id: reaction.message_id.get(),
          guild_id: reaction.guild_id.map(|id| id.get()),
          emoji: match &reaction.emoji {
            ReactionType::Custom { id, name, animated } => Some(Emoji {
              id: Some(id.get()),
              name: name.clone().unwrap(),
              animated: *animated,
            }),
            ReactionType::Unicode { name } => Some(Emoji {
              id: None,
              name: name.clone(),
              animated: false,
            }),
          },
        };
        let event = DiscordExEvent {
          discord_ex_event_union: Some(DiscordExEventUnion::ReactionCreate(data)),
        };
        event.encode_to_vec()
      })
      .await;
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
  Error,
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
      http
        .create_message(msg.channel_id)
        .content("compilation failed")
        .unwrap()
        .attachments(&attachments)?
        .await?;
    } else {
      http
        .create_message(msg.channel_id)
        .content(&format!("compilation failed: ```c\n{}```", compile_error))?
        .await?;
    }
    return Err(Box::new(CompileError::Error));
  }

  let assembly = get_disassembled(&binary_filename).await;
  let _assembly = Regex::new(r"(?m)^ffffffff80[0-9a-f]{6}")
    .unwrap()
    .replace_all(&assembly, |captures: &Captures| {
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

async fn dispatch_interrupt(contexts: &Arc<Contexts>, guild_id: Id<GuildMarker>, block: impl FnOnce(Arc<ExecutionContext>, &mut Cpu) -> Vec<u8>) {
  info!("dispatch int");
  let context = {
    let mut contexts = contexts.contexts.write().await;
    let context = contexts.entry(guild_id);
    context.or_insert_with(|| Arc::new(ExecutionContext::new())).clone()
  };
  let isolate = context.isolate.lock().await;
  let cpu = isolate.as_ref().unwrap().get_bootstrap_core();
  let mut cpu = cpu.lock().await;

  cpu.halt = false;

  let data = block(context.clone(), &mut cpu);
  cpu.bus.write(HARDWARE_BASE + 0x16000, &data).unwrap();
  cpu.regs[10] = data.len() as u64;
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
  WaitForInterrupt,
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
        if matches!(&exception, Exception::InstructionAccessFault(0)) {
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

    if let Some(interrupt) = self.check_pending_interrupt() {
      self.handle_interrupt(interrupt);
    }

    Ok(TickResult::Continue)
  }
}

async fn generate_rv_obj() -> (String, String, bool) {
  let output = Command::new("cargo")
    .env("CC", "/usr/bin/clang")
    .env("CXX", "/usr/bin/clang++")
    .current_dir("temp")
    .args(["+nightly", "build"])
    .output()
    .await
    .expect("Failed to generate rv object");
  let stderr = String::from_utf8_lossy(&output.stderr);
  println!("{}", stderr);
  (
    "target/riscv64g-unknown-mizu-elf/debug/temp".to_owned(),
    stderr.to_string(),
    output.status.success(),
  )
}

async fn generate_rv_binary(obj: &str) {
  let objcopy = "llvm-objcopy";
  let output = Command::new(objcopy)
    .args(["-O", "binary", obj, &format!("{}.bin", obj.to_owned())])
    .output()
    .await
    .expect("Failed to generate rv binary");
  println!("{}", String::from_utf8_lossy(&output.stderr));
}

async fn get_disassembled(obj: &str) -> String {
  let objcopy = "riscv64-unknown-elf-objdump";
  let output = Command::new(objcopy)
    .args([
      "--disassemble=_start",
      "--no-show-raw-insn",
      // "--visualize-jumps",
      // "--source",
      "-C",
      obj,
    ])
    .output()
    .await
    .expect("Failed to disassemble rv binary");
  return String::from_utf8_lossy(&output.stdout).to_string();
}
