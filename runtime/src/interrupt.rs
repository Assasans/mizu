pub const MASK_INTERRUPT_BIT: u64 = 1 << 63;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Interrupt {
  SupervisorSoftwareInterrupt,
  MachineSoftwareInterrupt,
  SupervisorTimerInterrupt,
  MachineTimerInterrupt,
  SupervisorExternalInterrupt,
  MachineExternalInterrupt,

  PlatformDefined16,
  PlatformDefined17,
}

impl Interrupt {
  pub fn code(&self) -> u64 {
    use Interrupt::*;
    match self {
      SupervisorSoftwareInterrupt => 1 | MASK_INTERRUPT_BIT,
      MachineSoftwareInterrupt => 3 | MASK_INTERRUPT_BIT,
      SupervisorTimerInterrupt => 5 | MASK_INTERRUPT_BIT,
      MachineTimerInterrupt => 7 | MASK_INTERRUPT_BIT,
      SupervisorExternalInterrupt => 9 | MASK_INTERRUPT_BIT,
      MachineExternalInterrupt => 11 | MASK_INTERRUPT_BIT,
      PlatformDefined16 => 16 | MASK_INTERRUPT_BIT,
      PlatformDefined17 => 17 | MASK_INTERRUPT_BIT,
    }
  }
}
