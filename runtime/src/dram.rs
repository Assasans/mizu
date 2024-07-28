use std::ops::Range;

use mizu_hwconst::memory::{DRAM_BASE, DRAM_SIZE};
use tracing::{error, warn};

use crate::exception::Exception;

pub struct Dram {
  pub dram: Vec<u8>,
  pub code_range: Range<u64>,
}

impl Dram {
  pub fn new(code: Vec<u8>) -> Self {
    let mut dram = vec![0; DRAM_SIZE as usize];
    dram.splice(..code.len(), code);
    Self {
      dram,
      code_range: DRAM_BASE..DRAM_BASE + 0x24000,
    }
  }

  // addr/size must be valid. Check in bus
  pub fn load(&self, addr: u64, size: u64) -> Result<u64, Exception> {
    if ![8, 16, 32, 64].contains(&size) {
      error!("unaligned load at 0x{addr:x}, {size}");
      return Err(Exception::LoadAccessFault(addr));
    }
    let nbytes = size / 8;
    let index = (addr - DRAM_BASE) as usize;
    let mut code = self.dram[index] as u64;
    // shift the bytes to build up the desired value
    for i in 1..nbytes {
      code |= (self.dram[index + i as usize] as u64) << (i * 8);
    }

    Ok(code)
  }

  // addr/size must be valid. Check in bus
  pub fn store(&mut self, addr: u64, size: u64, value: u64) -> Result<(), Exception> {
    if ![8, 16, 32, 64].contains(&size) {
      return Err(Exception::StoreAMOAccessFault(addr));
    }

    if self.code_range.contains(&addr) {
      warn!("tried to overwrite code segment at 0x{:x} with 0x{:x}", addr, value);
      return Err(Exception::StoreAMOAccessFault(addr));
    }

    let nbytes = size / 8;
    let index = (addr - DRAM_BASE) as usize;
    for i in 0..nbytes {
      let offset = 8 * i as usize;
      self.dram[index + i as usize] = ((value >> offset) & 0xff) as u8;
    }
    Ok(())
  }
}
