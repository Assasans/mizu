use std::fmt;

#[derive(Debug, Copy, Clone)]
pub enum Exception {
  InstructionAddrMisaligned(u64),
  InstructionAccessFault(u64),
  IllegalInstruction(u64),
  Breakpoint(u64),
  LoadAccessMisaligned(u64),
  LoadAccessFault(u64),
  StoreAMOAddrMisaligned(u64),
  StoreAMOAccessFault(u64),
  #[deprecated] EnvironmentCallFromUMode(u64),
  #[deprecated] EnvironmentCallFromSMode(u64),
  #[deprecated] EnvironmentCallFromMMode(u64),
  #[deprecated] InstructionPageFault(u64),
  #[deprecated] LoadPageFault(u64),
  #[deprecated] StoreAMOPageFault(u64),
}

impl fmt::Display for Exception {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    use Exception::*;
    match self {
      InstructionAddrMisaligned(addr) => write!(f, "Instruction address misaligned {:#x}", addr),
      InstructionAccessFault(addr) => write!(f, "Instruction access fault {:#x}", addr),
      IllegalInstruction(inst) => write!(f, "Illegal instruction {:#x}", inst),
      Breakpoint(pc) => write!(f, "Breakpoint {:#x}", pc),
      LoadAccessMisaligned(addr) => write!(f, "Load access {:#x}", addr),
      LoadAccessFault(addr) => write!(f, "Load access fault {:#x}", addr),
      StoreAMOAddrMisaligned(addr) => write!(f, "Store or AMO address misaliged {:#x}", addr),
      StoreAMOAccessFault(addr) => write!(f, "Store or AMO access fault {:#x}", addr),
      EnvironmentCallFromUMode(pc) => write!(f, "Environment call from U-mode {:#x}", pc),
      EnvironmentCallFromSMode(pc) => write!(f, "Environment call from S-mode {:#x}", pc),
      EnvironmentCallFromMMode(pc) => write!(f, "Environment call from M-mode {:#x}", pc),
      InstructionPageFault(addr) => write!(f, "Instruction page fault {:#x}", addr),
      LoadPageFault(addr) => write!(f, "Load page fault {:#x}", addr),
      StoreAMOPageFault(addr) => write!(f, "Store or AMO page fault {:#x}", addr),
    }
  }
}


impl Exception {
  pub fn value(self) -> u64 {
    use Exception::*;
    match self {
      InstructionAddrMisaligned(addr) => addr,
      InstructionAccessFault(addr) => addr,
      IllegalInstruction(inst) => inst,
      Breakpoint(pc) => pc,
      LoadAccessMisaligned(addr) => addr,
      LoadAccessFault(addr) => addr,
      StoreAMOAddrMisaligned(addr) => addr,
      StoreAMOAccessFault(addr) => addr,
      EnvironmentCallFromUMode(pc) => pc,
      EnvironmentCallFromSMode(pc) => pc,
      EnvironmentCallFromMMode(pc) => pc,
      InstructionPageFault(addr) => addr,
      LoadPageFault(addr) => addr,
      StoreAMOPageFault(addr) => addr,
    }
  }

  pub fn code(self) -> u64 {
    use Exception::*;
    match self {
      InstructionAddrMisaligned(_) => 0,
      InstructionAccessFault(_) => 1,
      IllegalInstruction(_) => 2,
      Breakpoint(_) => 3,
      LoadAccessMisaligned(_) => 4,
      LoadAccessFault(_) => 5,
      StoreAMOAddrMisaligned(_) => 6,
      StoreAMOAccessFault(_) => 7,
      EnvironmentCallFromUMode(_) => 8,
      EnvironmentCallFromSMode(_) => 9,
      EnvironmentCallFromMMode(_) => 11,
      InstructionPageFault(_) => 12,
      LoadPageFault(_) => 13,
      StoreAMOPageFault(_) => 15,
    }
  }

  pub fn is_fatal(self) -> bool {
    use Exception::*;
    match self {
      InstructionAddrMisaligned(_)
      | InstructionAccessFault(_)
      | LoadAccessFault(_)
      | StoreAMOAddrMisaligned(_)
      | StoreAMOAccessFault(_)
      | IllegalInstruction(_) => true,
      _else => false,
    }
  }
}
