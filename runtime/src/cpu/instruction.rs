use std::ops::Deref;

pub struct Instruction(pub u64);

impl Deref for Instruction {
  type Target = u64;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl Instruction {
  #[must_use]
  #[inline(always)]
  pub fn opcode(&self) -> u8 {
    (self.0 & 0x0000007f) as u8
  }

  #[must_use]
  #[inline(always)]
  pub fn rd(&self) -> usize {
    ((self.0 & 0x00000f80) >> 7) as usize
  }

  #[must_use]
  #[inline(always)]
  pub fn rs1(&self) -> usize {
    ((self.0 & 0x000f8000) >> 15) as usize
  }

  #[must_use]
  #[inline(always)]
  pub fn rs2(&self) -> usize {
    ((self.0 & 0x01f00000) >> 20) as usize
  }

  #[must_use]
  #[inline(always)]
  pub fn funct3(&self) -> u64 {
    (self.0 & 0x00007000) >> 12
  }

  #[must_use]
  #[inline(always)]
  pub fn funct7(&self) -> u64 {
    (self.0 & 0xfe000000) >> 25
  }
}
