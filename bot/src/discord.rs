use std::ffi::c_char;
use std::sync::Arc;
use async_trait::async_trait;
use hal_types::discord::{action, discord_create_message_t, discord_create_reaction_t, discord_message_t};
use tracing::debug;
use twilight_http::Client;
use twilight_http::request::channel::reaction::RequestReactionType;
use twilight_model::channel::message::MessageFlags;
use twilight_model::gateway::event::Event;
use twilight_model::id::Id;
use twilight_model::id::marker::{ChannelMarker, GuildMarker, StickerMarker};
use twilight_standby::Standby;
use runtime::bus::BusMemoryExt;
use runtime::cpu::{Cpu, InterruptHandler};
use runtime::param::DRAM_BASE;

pub struct DiscordInterruptHandler {
  pub guild_id: Id<GuildMarker>,
  pub channel_id: Id<ChannelMarker>,
  pub standby: Arc<Standby>,
  pub http: Arc<Client>,
}

#[async_trait]
impl InterruptHandler for DiscordInterruptHandler {
  async fn handle(&self, cpu: &mut Cpu) {
    let id = cpu.regs[10];
    let address = cpu.regs[11];
    debug!("discord call: id={} address=0x{:x}", id, address);

    match id {
      action::CREATE_MESSAGE => {
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

        let ffi_message = discord_message_t {
          id: response.id.get(),
          channel_id: response.channel_id.get(),
          author_id: response.author.id.get(),
          content: (DRAM_BASE + 0x9900) as *const c_char,
        };

        cpu.bus.write_string(ffi_message.content as u64, &response.content).unwrap();
        cpu.bus.write_struct(DRAM_BASE + 0x6000, &ffi_message).unwrap();
        cpu.regs[10] = DRAM_BASE + 0x6000;
      }
      action::CREATE_REACTION => {
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
