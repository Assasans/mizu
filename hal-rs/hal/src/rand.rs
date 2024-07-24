use core::ptr;

pub use rand_core::RngCore;

pub const RANDOM_BASE: *const u8 = 0x12000 as *const u8;
pub const RANDOM_SIZE: usize = 0x100;

pub struct MemoryMappedRng {
  address: *const u8
}

impl MemoryMappedRng {
  pub fn new() -> Self {
    MemoryMappedRng { address: RANDOM_BASE }
  }
}

impl RngCore for MemoryMappedRng {
  fn next_u32(&mut self) -> u32 {
    unsafe { ptr::read_volatile(self.address as *const u32) }
  }

  fn next_u64(&mut self) -> u64 {
    unsafe { ptr::read_volatile(self.address as *const u64) }
  }

  fn fill_bytes(&mut self, dest: &mut [u8]) {
    for byte in dest {
      *byte = unsafe { ptr::read_volatile(self.address) };
    }
  }

  fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
    Ok(self.fill_bytes(dest))
  }
}
