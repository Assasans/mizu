use std::time::SystemTime;

use async_trait::async_trait;
use runtime::cpu::{Cpu, InterruptHandler};

pub struct TimeHandler {}

/// Upper 64 bits go to `a0`, lower to `a1`
#[async_trait]
impl InterruptHandler for TimeHandler {
  async fn handle(&self, cpu: &mut Cpu) {
    let time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    let nanos = time.as_nanos();

    cpu.regs[10] = (nanos >> 64) as u64;
    cpu.regs[11] = nanos as u64;
  }
}
