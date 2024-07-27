use std::time::SystemTime;

use async_trait::async_trait;
use runtime::cpu::{Cpu, InterruptHandler};

pub struct TimeHandler {}

#[async_trait]
impl InterruptHandler for TimeHandler {
  async fn handle(&self, cpu: &mut Cpu) {
    let time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    cpu.regs[10] = time.as_nanos() as u64;
  }
}
