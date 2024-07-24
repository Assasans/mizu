use std::time::Duration;
use mizu_hwconst::memory::CPUID_BASE;
pub use mizu_hwconst::csr::*;

pub struct Csr {
  csrs: [u64; NUM_CSRS],
  time_passed: Box<dyn Fn() -> Duration + Send + Sync>
}

impl Csr {
  pub fn new(time_passed: Box<dyn Fn() -> Duration + Send + Sync>) -> Csr {
    Self {
      csrs: [0; NUM_CSRS],
      time_passed
    }
  }

  pub fn dump_csrs(&self) -> String {
    format!(
      "// control status registers\n{}\n{}\n",
      format!(
        "mstatus = {:<#18x}  mtvec = {:<#18x}  mepc = {:<#18x}  mcause = {:<#18x}",
        self.load(MSTATUS),
        self.load(MTVEC),
        self.load(MEPC),
        self.load(MCAUSE),
      ),
      format!(
        "sstatus = {:<#18x}  stvec = {:<#18x}  sepc = {:<#18x}  scause = {:<#18x}",
        self.load(SSTATUS),
        self.load(STVEC),
        self.load(SEPC),
        self.load(SCAUSE),
      ),
    )
  }

  pub fn load(&self, addr: usize) -> u64 {
    match addr {
      SIE => self.csrs[MIE] & self.csrs[MIDELEG],
      SIP => self.csrs[MIP] & self.csrs[MIDELEG],
      SSTATUS => self.csrs[MSTATUS] & MASK_SSTATUS,
      machine::CONFIGPTR => CPUID_BASE,
      unprivileged::TIME => (self.time_passed)().as_nanos() as u64,
      _ => self.csrs[addr],
    }
  }

  pub fn store(&mut self, addr: usize, value: u64) {
    match addr {
      SIE => self.csrs[MIE] = (self.csrs[MIE] & !self.csrs[MIDELEG]) | (value & self.csrs[MIDELEG]),
      SIP => self.csrs[MIP] = (self.csrs[MIE] & !self.csrs[MIDELEG]) | (value & self.csrs[MIDELEG]),
      SSTATUS => self.csrs[MSTATUS] = (self.csrs[MSTATUS] & !MASK_SSTATUS) | (value & MASK_SSTATUS),
      _ => self.csrs[addr] = value,
    }
  }

  #[inline]
  pub fn is_medelegated(&self, cause: u64) -> bool {
    (self.csrs[MEDELEG].wrapping_shr(cause as u32) & 1) == 1
  }

  #[inline]
  pub fn is_midelegated(&self, cause: u64) -> bool {
    (self.csrs[MIDELEG].wrapping_shr(cause as u32) & 1) == 1
  }
}
