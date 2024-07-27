pub mod apic;
pub mod bus;
pub mod cpu;
pub mod csr;
pub mod dram;
pub mod exception;
pub mod interrupt;
pub mod isolate;
pub mod memory;
pub mod perf_counter;
pub mod state_flow;

#[cfg(test)]
mod tests {
  use std::fs;
  use std::sync::Arc;

  use tracing::{error, info};
  use tracing_subscriber::prelude::*;
  use tracing_subscriber::{fmt, EnvFilter};

  use crate::bus::Bus;
  use crate::cpu::Cpu;

  #[tokio::test]
  async fn main() {
    tracing_subscriber::registry().with(fmt::layer()).with(EnvFilter::from_default_env()).init();

    info!("Hello, world!");
    let code = fs::read("../bot/temp/main.bin").unwrap();
    // let code = fs::read("hal.bin").unwrap();
    let bus = Arc::new(Bus::new(code));
    let mut cpu = Cpu::new(0, bus, None);
    loop {
      let inst = match cpu.fetch() {
        Ok(inst) => inst,
        Err(exception) => {
          cpu.handle_exception(exception);
          if exception.is_fatal() {
            error!("fetch failed: {:?}", exception);
            break;
          }
          continue;
        }
      };

      match cpu.execute(inst).await {
        Ok(new_pc) => cpu.pc = new_pc,
        Err(exception) => {
          cpu.handle_exception(exception);
          if exception.is_fatal() {
            error!("execute failed: {:?}", exception);
            break;
          }
        }
      };

      match cpu.check_pending_interrupt() {
        Some(interrupt) => cpu.handle_interrupt(interrupt),
        None => (),
      }
    }
  }
}
