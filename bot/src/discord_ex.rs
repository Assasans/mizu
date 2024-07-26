use std::ffi::c_char;
use std::sync::Arc;
use async_trait::async_trait;
use mizu_hal_types::discord::{action, discord_create_message_t, discord_create_reaction_t, discord_get_user_t, discord_message_t, discord_user_t};
use tracing::debug;
use twilight_http::Client;
use twilight_http::request::channel::reaction::RequestReactionType;
use twilight_model::channel::message::MessageFlags;
use twilight_model::gateway::event::Event;
use twilight_model::id::Id;
use twilight_model::id::marker::{ChannelMarker, GuildMarker, StickerMarker};
use twilight_standby::Standby;
use mizu_hal_discord::discord::discord_ex_request::DiscordExRequestUnion;
use mizu_hal_discord::prost::Message;
use mizu_hal_discord::discord::DiscordExRequest;
use mizu_hal_types::StringPtr;
use runtime::bus::{Bus, BusMemoryExt};
use runtime::cpu::{Cpu, InterruptHandler};
use runtime::memory::HARDWARE_BASE;
use crate::execution_context::ExecutionContext;

pub struct DiscordExInterruptHandler {
  pub context: Arc<ExecutionContext>,
  pub guild_id: Id<GuildMarker>,
  pub standby: Arc<Standby>,
}

#[async_trait]
impl InterruptHandler for DiscordExInterruptHandler {
  async fn handle(&self, cpu: &mut Cpu) {
    let length = cpu.regs[10];
    let address = cpu.regs[11];
    debug!("discord call: length={} address=0x{:x}", length, address);

    let request = cpu.bus.read(address, length).unwrap();
    debug!("request: {:?}", request);

    let http = self.context.http.lock().await.as_ref().unwrap().clone();
    let request = DiscordExRequest::decode(&*request).unwrap();
    match request.discord_ex_request_union.unwrap() {
      DiscordExRequestUnion::CreateMessageRequest(create_message) => {
        let mut builder = http.create_message(Id::new(create_message.channel_id));
        if let Some(content) = create_message.content.as_deref() {
          builder = builder.content(content).unwrap();
        }

        if let Some(reference_id) = create_message.reference_id {
          builder = builder.reply(Id::new(reference_id));
        }

        let response = builder
          .await.unwrap()
          .model().await.unwrap();
        cpu.regs[10] = response.id.get();
      }
      DiscordExRequestUnion::EditMessageRequest(edit_message) => {
        let mut builder = http.update_message(Id::new(edit_message.channel_id), Id::new(edit_message.message_id));
        builder = builder.content(edit_message.content.as_deref()).unwrap();

        let response = builder
          .await.unwrap()
          .model().await.unwrap();
        cpu.regs[10] = response.id.get();
      }
    }
  }
}
