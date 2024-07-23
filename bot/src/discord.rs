use std::ffi::c_char;
use std::sync::Arc;
use async_trait::async_trait;
use hal_types::discord::{action, discord_create_message_t, discord_create_reaction_t, discord_get_user_t, discord_message_t, discord_user_t};
use tracing::debug;
use twilight_http::Client;
use twilight_http::request::channel::reaction::RequestReactionType;
use twilight_model::channel::message::MessageFlags;
use twilight_model::gateway::event::Event;
use twilight_model::id::Id;
use twilight_model::id::marker::{ChannelMarker, GuildMarker, StickerMarker};
use twilight_standby::Standby;
use hal_types::StringPtr;
use runtime::bus::{Bus, BusMemoryExt};
use runtime::cpu::{Cpu, InterruptHandler};
use runtime::param::DRAM_BASE;
use crate::execution_context::ExecutionContext;

pub struct DiscordInterruptHandler {
  pub context: Arc<ExecutionContext>,
  pub guild_id: Id<GuildMarker>,
  pub standby: Arc<Standby>,
}

pub trait MemoryObject<T> {
  fn read(&self, bus: &Bus) -> T;
  fn write(&self, bus: &Bus, value: &T);
}

impl MemoryObject<String> for StringPtr {
  fn read(&self, bus: &Bus) -> String {
    bus.read_string(self.0 as u64).unwrap().to_str().unwrap().to_owned()
  }

  fn write(&self, bus: &Bus, value: &String) {
    bus.write_string(self.0 as u64, value).unwrap();
  }
}

#[async_trait]
impl InterruptHandler for DiscordInterruptHandler {
  async fn handle(&self, cpu: &mut Cpu) {
    let id = cpu.regs[10];
    let address = cpu.regs[11];
    debug!("discord call: id={} address=0x{:x}", id, address);

    let http = self.context.http.lock().await.as_ref().unwrap().clone();
    match id {
      action::CREATE_MESSAGE => {
        let request = cpu.bus.read_struct::<discord_create_message_t>(address).unwrap();
        debug!("request: {:?}", request);

        let mut builder = http.create_message(Id::new(request.channel_id));

        let content = if !request.content.is_null() {
          Some(request.content.read(&mut cpu.bus))
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

        let ffi_message = discord_message_t {
          id: response.id.get(),
          channel_id: response.channel_id.get(),
          author_id: response.author.id.get(),
          content: StringPtr((DRAM_BASE + 0x9900) as *const c_char),
        };

        ffi_message.content.write(&cpu.bus, &response.content);
        cpu.bus.write_struct(DRAM_BASE + 0x6000, &ffi_message).unwrap();
        cpu.regs[10] = DRAM_BASE + 0x6000;
      }
      action::CREATE_REACTION => {
        let request = cpu.bus.read_struct::<discord_create_reaction_t>(address).unwrap();
        debug!("request: {:?}", request);

        http.create_reaction(
          Id::new(request.channel_id),
          Id::new(request.message_id),
          &RequestReactionType::Unicode { name: &request.emoji.read(&cpu.bus) },
        ).await.unwrap();
        cpu.regs[10] = 0;
      }
      action::GET_USER => {
        let request = cpu.bus.read_struct::<discord_get_user_t>(address).unwrap();
        debug!("request: {:?}", request);

        let response = http.user(Id::new(request.user_id))
          .await.unwrap()
          .model().await.unwrap();

        let ffi_user = discord_user_t {
          id: response.id.get(),
          name: StringPtr((DRAM_BASE + 0x8800) as *const c_char),
          global_name: StringPtr((DRAM_BASE + 0x9900) as *const c_char),
        };

        ffi_user.name.write(&cpu.bus, &response.name);
        ffi_user.global_name.write(&cpu.bus, response.global_name.as_ref().unwrap());
        cpu.bus.write_struct(DRAM_BASE + 0x6000, &ffi_user).unwrap();
        cpu.regs[10] = DRAM_BASE + 0x6000;
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
          content: StringPtr((DRAM_BASE + 0x9900) as *const c_char),
        };

        ffi_message.content.write(&mut cpu.bus, &message.content);
        cpu.bus.write_struct(DRAM_BASE + 0x6000, &ffi_message).unwrap();
        cpu.regs[10] = DRAM_BASE + 0x6000;
      }
      _ => unimplemented!()
    }
  }
}
