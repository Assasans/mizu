pub mod cpu;
pub mod bus;
pub mod dram;
pub mod param;
pub mod exception;
pub mod csr;

#[cfg(test)]
mod tests {
  use std::fs;
  use tracing::{debug, error, info, trace};
  use tracing_subscriber::{EnvFilter, fmt, prelude::*};
  use crate::cpu::Cpu;

  #[test]
  fn main() {
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
          error!("fetch failed: {:?}", exception);
          break;
        }
      };

      match cpu.execute(inst) {
        Ok(new_pc) => cpu.pc = new_pc,
        Err(exception) => {
          error!("execute failed: {:?}", exception);
          break;
        }
      };
    }
  }
}
