pub mod cpu;
pub mod bus;
pub mod dram;
pub mod param;
pub mod exception;
pub mod csr;
pub mod interrupt;
pub mod perf_counter;

#[cfg(test)]
mod tests {
  use std::fs;
  use tracing::{debug, error, info, trace};
  use tracing_subscriber::{EnvFilter, fmt, prelude::*};
  use crate::cpu::Cpu;

  #[tokio::test]
  async fn main() {
    tracing_subscriber::registry()
      .with(fmt::layer())
      .with(EnvFilter::from_default_env())
      .init();

    info!("Hello, world!");
    let code = fs::read("../bot/temp/main.bin").unwrap();
    // let code = fs::read("hal.bin").unwrap();
    let mut cpu = Cpu::new(code);
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
