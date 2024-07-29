use std::sync::Arc;

pub use mizu_hwconst::csr::*;
use mizu_hwconst::memory::CPUID_BASE;
use crate::perf_counter::PerformanceCounter;

/// Control and status registers. RISC-V ISA sets aside a 12-bit encoding space (csr[11:0]) for
/// up to 4096 CSRs.
pub struct Csr {
  csrs: [u64; NUM_CSRS],
  perf: Arc<PerformanceCounter>,
}

impl Csr {
  #[must_use]
  pub fn new(perf: Arc<PerformanceCounter>) -> Self {
    Self {
      csrs: [0; NUM_CSRS],
      perf,
    }
  }

  #[must_use]
  pub fn dump_csrs(&self) -> String {
    format!(
      "// control status registers\n{}\n{}\n",
      format_args!(
        "mstatus = {:<#18x}  mtvec = {:<#18x}  mepc = {:<#18x}  mcause = {:<#18x}",
        self.load(MSTATUS),
        self.load(MTVEC),
        self.load(MEPC),
        self.load(MCAUSE),
      ),
      format_args!(
        "sstatus = {:<#18x}  stvec = {:<#18x}  sepc = {:<#18x}  scause = {:<#18x}",
        self.load(SSTATUS),
        self.load(STVEC),
        self.load(SEPC),
        self.load(SCAUSE),
      ),
    )
  }

  #[must_use]
  pub fn load(&self, addr: usize) -> u64 {
    match addr {
      SIE => self.csrs[MIE] & self.csrs[MIDELEG],
      SIP => self.csrs[MIP] & self.csrs[MIDELEG],
      SSTATUS => self.csrs[MSTATUS] & MASK_SSTATUS,
      machine::CONFIGPTR => CPUID_BASE,
      unprivileged::TIME => self.perf.cpu_time.lock().unwrap().as_nanos() as u64,
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
  #[must_use]
  pub const fn is_medelegated(&self, cause: u64) -> bool {
    (self.csrs[MEDELEG].wrapping_shr(cause as u32) & 1) == 1
  }

  #[inline]
  #[must_use]
  pub const fn is_midelegated(&self, cause: u64) -> bool {
    (self.csrs[MIDELEG].wrapping_shr(cause as u32) & 1) == 1
  }
}
