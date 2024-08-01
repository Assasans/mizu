use std::sync::Arc;

use async_trait::async_trait;
use runtime::bus::BusMemoryExt;
use runtime::cpu::{Cpu, InterruptHandler};
use tracing::debug;

use crate::execution_context::ExecutionContext;

pub struct LogHandler {
  pub context: Arc<ExecutionContext>,
}

#[async_trait]
impl InterruptHandler for LogHandler {
  async fn handle(&self, cpu: &mut Cpu) {
    let http = self.context.http.lock().await.as_ref().unwrap().clone();
    let channel_id = self.context.channel_id.lock().await.unwrap();

    let address = cpu.regs[10];
    debug!("log address: 0x{:x}", address);
    let message = cpu.bus.read_string(address).unwrap().to_string_lossy().to_string();
    debug!("log message: {}", message);

    let cpu_id = cpu.id;
    tokio::spawn(async move {
      http
        .create_message(channel_id)
        .content(&format!("sys_print cpu {}: `{}`", cpu_id, message))
        .unwrap()
        .await
        .unwrap();
    });
  }
}
