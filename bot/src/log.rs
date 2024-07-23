use std::sync::Arc;
use async_trait::async_trait;
use runtime::bus::BusMemoryExt;
use tracing::debug;
use twilight_http::Client;
use twilight_model::id::Id;
use twilight_model::id::marker::{ChannelMarker, GuildMarker};
use twilight_standby::Standby;
use runtime::cpu::{Cpu, InterruptHandler};

pub struct LogHandler {
  pub guild_id: Id<GuildMarker>,
  pub channel_id: Id<ChannelMarker>,
  pub standby: Arc<Standby>,
  pub http: Arc<Client>,
}

#[async_trait]
impl InterruptHandler for LogHandler {
  async fn handle(&self, cpu: &mut Cpu) {
    let address = cpu.regs[10];
    debug!("log address: 0x{:x}", address);
    let message = cpu.bus.read_string(address).unwrap().to_string_lossy().to_string();
    debug!("log message: {}", message);
    self.http.create_message(self.channel_id)
      .content(&format!("sys_print: `{}`", message)).unwrap()
      .await.unwrap();
  }
}
