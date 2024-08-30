use std::ops::Range;

use mizu_hwconst::memory::DRAM_SIZE;
use tracing::{error, warn};

use crate::exception::Exception;

pub struct Dram {
  pub dram: Vec<u8>,
}

impl Dram {
  pub fn new(code: Vec<u8>) -> Self {
    let mut dram = vec![0; DRAM_SIZE as usize];
    dram.splice(..code.len(), code);
    Self { dram }
  }

  pub fn load(&self, addr: u64, size: u64) -> Result<u64, Exception> {
    let index = addr as usize;
    match size {
      8 => Ok(self.dram[index] as u64),
      16 => Ok(u16::from_le_bytes(self.dram[index..index + 2].try_into().unwrap()) as u64),
      32 => Ok(u32::from_le_bytes(self.dram[index..index + 4].try_into().unwrap()) as u64),
      64 => Ok(u64::from_le_bytes(self.dram[index..index + 8].try_into().unwrap())),
      _ => {
        error!("unaligned load at 0x{addr:x}, {size}");
        Err(Exception::LoadAccessFault(addr))
      }
    }
  }

  pub fn store(&mut self, addr: u64, size: u64, value: u64) -> Result<(), Exception> {
    let index = addr as usize;
    match size {
      8 => self.dram[index] = value as u8,
      16 => self.dram[index..index + 2].copy_from_slice(&u16::to_le_bytes(value as u16)),
      32 => self.dram[index..index + 4].copy_from_slice(&u32::to_le_bytes(value as u32)),
      64 => self.dram[index..index + 8].copy_from_slice(&u64::to_le_bytes(value)),
      _ => {
        error!("unaligned store at 0x{addr:x}, {size}");
        return Err(Exception::StoreAMOAccessFault(addr));
      }
    }

    Ok(())
  }
}
