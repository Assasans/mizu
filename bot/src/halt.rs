use async_trait::async_trait;
use runtime::cpu::{Cpu, InterruptHandler};
use tracing::debug;

pub struct HaltHandler {
}

#[async_trait]
impl InterruptHandler for HaltHandler {
  async fn handle(&self, cpu: &mut Cpu) {
    debug!("halting execution...");
    cpu.halt = true;
  }
}
