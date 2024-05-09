use std::sync::Arc;
use async_trait::async_trait;
use twilight_http::Client;
use twilight_model::id::Id;
use twilight_model::id::marker::{ChannelMarker, GuildMarker};
use twilight_standby::Standby;
use runtime::cpu::{Cpu, InterruptHandler};

pub struct DumpPerformanceHandler {
  pub guild_id: Id<GuildMarker>,
  pub channel_id: Id<ChannelMarker>,
  pub standby: Arc<Standby>,
  pub http: Arc<Client>,
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
