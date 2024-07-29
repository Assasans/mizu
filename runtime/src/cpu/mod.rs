mod amo;
mod auipc;
mod branch;
mod instruction;
mod jal;
mod jalr;
mod load;
mod load_fp;
mod lui;
mod op;
mod op_32;
mod op_fp;
mod op_imm;
mod op_imm_32;
mod store;
mod store_fp;
mod system;

use std::collections::HashMap;
use std::fmt::Write;
use std::sync::{Arc, Weak};
use std::sync::atomic::Ordering;
use std::time::Instant;

use async_trait::async_trait;
pub use instruction::Instruction;
use mizu_hwconst::memory::DRAM_BASE;
use tracing::{debug, info, trace};

use crate::apic::Apic;
use crate::bus::Bus;
use crate::cpu::amo::amo;
use crate::cpu::auipc::auipc;
use crate::cpu::branch::branch;
use crate::cpu::jal::jal;
use crate::cpu::jalr::jalr;
use crate::cpu::load::load;
use crate::cpu::load_fp::load_fp;
use crate::cpu::lui::lui;
use crate::cpu::op::op;
use crate::cpu::op_32::op_32;
use crate::cpu::op_fp::op_fp;
use crate::cpu::op_imm::op_imm;
use crate::cpu::op_imm_32::opp_imm_32;
use crate::cpu::store::store;
use crate::cpu::store_fp::store_fp;
use crate::cpu::system::system;
use crate::csr;
use crate::csr::{
  Csr, MASK_MEIP, MASK_MIE, MASK_MPIE, MASK_MPP, MASK_MSIP, MASK_MTIP, MASK_SEIP, MASK_SSIP, MASK_STIP, MCAUSE, MEPC, MIE, MIP, MSTATUS, MTVAL, MTVEC,
};
use crate::exception::Exception;
use crate::interrupt::Interrupt;
use crate::isolate::Isolate;
use crate::perf_counter::PerformanceCounter;
use crate::state_flow::StateFlow;

#[async_trait]
pub trait InterruptHandler: Send + Sync {
  async fn handle(&self, cpu: &mut Cpu);
}

pub struct Cpu {
  pub id: u16,
  pub isolate: Option<Weak<Isolate>>,
  pub regs: [u64; 32],
  pub saved_regs: [u64; 32],
  pub fp_regs: [f64; 32],
  pub pc: u64,
  pub bus: Arc<Bus>,
  pub apic: Apic,
  pub csr: Csr,
  pub ivt: HashMap<u64, Arc<Box<dyn InterruptHandler>>>,
  pub perf: Arc<PerformanceCounter>,
  pub halt: bool,
  pub wfi: StateFlow<bool>,
}

impl Cpu {
  pub fn new(id: u16, bus: Arc<Bus>, isolate: Option<Weak<Isolate>>) -> Self {
    let registers = [0; 32];

    // Set the register x2 with the size of a memory when a CPU is instantiated.
    // registers[2] = DRAM_BASE + 0x1000000 + (0x20000 * (id + 1) as u64);
    debug!("initialized sp=0x{:x}", registers[2]);

    let perf = Arc::new(PerformanceCounter::new());

    let pc = DRAM_BASE;

    let mut csr = Csr::new(perf.clone());
    csr.store(csr::machine::POWERSTATE, 1);

    let apic = Apic::new();
    let ivt = HashMap::new();

    Self {
      id,
      isolate,
      regs: registers,
      saved_regs: [0; 32],
      fp_regs: [0.0; 32],
      pc,
      bus,
      apic,
      csr,
      ivt,
      perf,
      halt: false,
      wfi: StateFlow::new(false),
    }
  }

  /// Load a value from a dram.
  pub fn load(&mut self, addr: u64, size: u64) -> Result<u64, Exception> {
    self.bus.load(addr, size)
  }

  /// Store a value to a dram.
  pub fn store(&mut self, addr: u64, size: u64, value: u64) -> Result<(), Exception> {
    self.bus.store(addr, size, value)
  }

  /// Get an instruction from the dram.
  pub fn fetch(&mut self) -> Result<u64, Exception> {
    // trace!("fetching instruction...");
    match self.bus.load(self.pc, 32) {
      Ok(inst) => Ok(inst),
      Err(_e) => Err(Exception::InstructionAccessFault(self.pc)),
    }
  }

  pub fn dump(&self) -> String {
    let mut output = String::new();
    output.write_fmt(format_args!("cpu={:<#18}\n", self.id)).unwrap();
    output
      .write_fmt(format_args!(
        "cpu_time={:<#18?} insts_retired={}\n",
        self.perf.cpu_time.lock().unwrap(), self.perf.instructions_retired.load(Ordering::Acquire)
      ))
      .unwrap();
    output
      .write_fmt(format_args!("pc={:<#18x}       mepc={:<#18x}\n", self.pc, self.csr.load(MEPC)))
      .unwrap();

    let registers = [(1, "ra"), (2, "sp"), (10, "a0"), (17, "a7")];
    for chunk in registers.chunks(4) {
      output.push_str(
        &chunk
          .iter()
          .map(|(index, name)| {
            let value = self.regs[*index];
            format!("x{:02}→{}={:<#18x}", index, name, value)
          })
          .map(|it| format!("{:<26}", it))
          .collect::<Vec<_>>()
          .join("  "),
      );
      output.push('\n');
    }

    let csrs = [
      (MSTATUS, "mstatus"),
      (MTVEC, "mtvec"),
      (MCAUSE, "mcause"),
      (csr::machine::POWERSTATE, "mpowerstate"),
    ];
    for chunk in csrs.chunks(4) {
      output.push_str(
        &chunk
          .iter()
          .map(|(address, name)| {
            let value = self.csr.load(*address);
            format!("{}={:<#18x}", name, value)
          })
          .map(|it| format!("{:<26}", it))
          .collect::<Vec<_>>()
          .join("  "),
      );
      output.push('\n');
    }

    output
  }

  pub fn dump_registers(&self) -> String {
    let mut output = String::new();
    let abi = [
      "zero", " ra ", " sp ", " gp ", " tp ", " t0 ", " t1 ", " t2 ", " s0 ", " s1 ", " a0 ", " a1 ", " a2 ", " a3 ", " a4 ", " a5 ", " a6 ", " a7 ", " s2 ",
      " s3 ", " s4 ", " s5 ", " s6 ", " s7 ", " s8 ", " s9 ", " s10", " s11", " t3 ", " t4 ", " t5 ", " t6 ",
    ];
    for i in (0..32).step_by(4) {
      output
        .write_fmt(format_args!(
          "\nx{:02}({})={:>#18x} x{:02}({})={:>#18x} x{:02}({})={:>#18x} x{:02}({})={:>#18x}",
          i,
          abi[i],
          self.regs[i],
          i + 1,
          abi[i + 1],
          self.regs[i + 1],
          i + 2,
          abi[i + 2],
          self.regs[i + 2],
          i + 3,
          abi[i + 3],
          self.regs[i + 3],
        ))
        .unwrap();
    }
    output
  }

  pub fn handle_exception(&mut self, exception: Exception) {
    // the process to handle exception in S-mode and M-mode is similar,
    // includes following steps:
    // 0. set xPP to current mode.
    // 1. update hart's privilege mode (M or S according to current mode and exception setting).
    // 2. save current pc in epc (sepc in S-mode, mepc in M-mode)
    // 3. set pc to trap vector (stvec in S-mode, mtvec in M-mode)
    // 4. set cause to exception code (scause in S-mode, mcause in M-mode)
    // 5. set trap value properly (stval in S-mode, mtval in M-mode)
    // 6. set xPIE to xIE (SPIE in S-mode, MPIE in M-mode)
    // 7. clear up xIE (SIE in S-mode, MIE in M-mode)
    let pc = self.pc;
    let cause = exception.code();

    // 3.1.7 & 4.1.2
    // The BASE field in tvec is a WARL field that can hold any valid virtual or physical address,
    // subject to the following alignment constraints: the address must be 4-byte aligned
    self.pc = self.csr.load(MTVEC) & !0b11;
    // 3.1.14 & 4.1.7
    // When a trap is taken into S-mode (or M-mode), sepc (or mepc) is written with the virtual address
    // of the instruction that was interrupted or that encountered the exception.
    self.csr.store(MEPC, pc);
    // 3.1.15 & 4.1.8
    // When a trap is taken into S-mode (or M-mode), scause (or mcause) is written with a code indicating
    // the event that caused the trap.
    self.csr.store(MCAUSE, cause);
    // 3.1.16 & 4.1.9
    // If stval is written with a nonzero value when a breakpoint, address-misaligned, access-fault, or
    // page-fault exception occurs on an instruction fetch, load, or store, then stval will contain the
    // faulting virtual address.
    // If stval is written with a nonzero value when a misaligned load or store causes an access-fault or
    // page-fault exception, then stval will contain the virtual address of the portion of the access that
    // caused the fault
    self.csr.store(MTVAL, exception.value());
    // 3.1.6 covers both sstatus and mstatus.
    let mut status = self.csr.load(MSTATUS);
    // get SIE or MIE
    let ie = (status & MASK_MIE) >> 3;
    // set SPIE = SIE / MPIE = MIE
    status = (status & !MASK_MPIE) | (ie << 7);
    // set SIE = 0 / MIE = 0
    status &= !MASK_MIE;
    // set SPP / MPP = previous mode
    status = (status & !MASK_MPP) | (0b11 << 11);
    self.csr.store(MSTATUS, status);
  }

  pub fn handle_interrupt(&mut self, interrupt: Interrupt) {
    // similar to handle exception
    let pc = self.pc;
    let cause = interrupt.code();

    // Save registers
    self.saved_regs.copy_from_slice(&self.regs);

    // 3.1.7 & 4.1.2
    // When MODE=Direct, all traps into machine mode cause the pc to be set to the address in the BASE field.
    // When MODE=Vectored, all synchronous exceptions into machine mode cause the pc to be set to the address
    // in the BASE field, whereas interrupts cause the pc to be set to the address in the BASE field plus four
    // times the interrupt cause number.
    let tvec = self.csr.load(MTVEC);
    let tvec_mode = tvec & 0b11;
    let tvec_base = tvec & !0b11;
    match tvec_mode {
      0 => self.pc = tvec_base,                // Direct
      1 => self.pc = tvec_base + (cause << 2), // Vector
      _ => unreachable!(),
    };
    debug!(
      "interrupt handler at 0x{:x}, base: 0x{:x}, mode: {}, cause offset: 0x{:x}, pc: 0x{:x}",
      self.pc,
      tvec_base,
      tvec_mode,
      cause << 2,
      pc
    );
    // 3.1.14 & 4.1.7
    // When a trap is taken into S-mode (or M-mode), sepc (or mepc) is written with the virtual address
    // of the instruction that was interrupted or that encountered the exception.
    self.csr.store(MEPC, pc);
    // 3.1.15 & 4.1.8
    // When a trap is taken into S-mode (or M-mode), scause (or mcause) is written with a code indicating
    // the event that caused the trap.
    self.csr.store(MCAUSE, cause);
    // 3.1.16 & 4.1.9
    // When a trap is taken into M-mode, mtval is either set to zero or written with exception-specific
    // information to assist software in handling the trap.
    self.csr.store(MTVAL, 0);
    // 3.1.6 covers both sstatus and mstatus.
    let mut status = self.csr.load(MSTATUS);
    // get SIE or MIE
    let ie = (status & MASK_MIE) >> 3;
    // set SPIE = SIE / MPIE = MIE
    status = (status & !MASK_MPIE) | (ie << 7);
    // set SIE = 0 / MIE = 0
    status &= !MASK_MIE;
    // set SPP / MPP = previous mode
    // status = (status & !MASK_MPP) | (3 << 11);
    self.csr.store(MSTATUS, status);
  }

  pub fn check_pending_interrupt(&mut self) -> Option<Interrupt> {
    use Interrupt::*;
    // 3.1.9 & 4.1.3
    // Multiple simultaneous interrupts destined for M-mode are handled in the following decreasing
    // priority order: MEI, MSI, MTI, SEI, SSI, STI.
    let pending = self.csr.load(MIE) & self.csr.load(MIP);

    if (pending & MASK_MEIP) != 0 {
      self.csr.store(MIP, self.csr.load(MIP) & !MASK_MEIP);
      return Some(MachineExternalInterrupt);
    }
    if (pending & MASK_MSIP) != 0 {
      self.csr.store(MIP, self.csr.load(MIP) & !MASK_MSIP);
      return Some(MachineSoftwareInterrupt);
    }
    if (pending & MASK_MTIP) != 0 {
      self.csr.store(MIP, self.csr.load(MIP) & !MASK_MTIP);
      return Some(MachineTimerInterrupt);
    }
    if (pending & MASK_SEIP) != 0 {
      self.csr.store(MIP, self.csr.load(MIP) & !MASK_SEIP);
      return Some(SupervisorExternalInterrupt);
    }
    if (pending & MASK_SSIP) != 0 {
      self.csr.store(MIP, self.csr.load(MIP) & !MASK_SSIP);
      return Some(SupervisorSoftwareInterrupt);
    }
    if (pending & MASK_STIP) != 0 {
      self.csr.store(MIP, self.csr.load(MIP) & !MASK_STIP);
      return Some(SupervisorTimerInterrupt);
    }

    let interrupt = self.apic.get();
    if interrupt.is_some() {
      info!("apic interrupt: {:?}", interrupt);
    }
    interrupt
  }

  #[inline]
  pub fn update_pc(&mut self) -> Result<u64, Exception> {
    Ok(self.pc + 4)
  }

  pub async fn execute(&mut self, inst: u64) -> Result<u64, Exception> {
    self.perf.start_cpu_time();
    let opcode = inst & 0x0000007f;
    let rd = ((inst & 0x00000f80) >> 7) as usize;
    let rs1 = ((inst & 0x000f8000) >> 15) as usize;
    let rs2 = ((inst & 0x01f00000) >> 20) as usize;
    let funct3 = (inst & 0x00007000) >> 12;
    let funct7 = (inst & 0xfe000000) >> 25;

    let instruction = Instruction(inst);

    // Emulate that register x0 is hardwired with all bits equal to 0.
    self.regs[0] = 0;

    trace!(
      "pc=0x{:x} ra=0x{:x} sp=0x{:x} opcode=0b{opcode:07b} ({opcode:x}) rd=0b{rd:05b} rs1=0b{rs1:05b} rs2=0b{rs2:05b} funct3=0b{funct3:03b} funct7=0b{funct7:03b}",
      self.pc,
      self.regs[1],
      self.regs[2]
    );

    // let opcode = Opcode::from(opcode);
    // trace!("executing opcode {:?}", opcode);

    match opcode {
      opcode::LOAD => load(instruction, self),
      opcode::LOAD_FP => load_fp(instruction, self),
      opcode::OP_IMM => op_imm(instruction, self),
      opcode::AUIPC => auipc(instruction, self),
      opcode::OP_IMM_32 => opp_imm_32(instruction, self),
      opcode::STORE => store(instruction, self),
      opcode::STORE_FP => store_fp(instruction, self),
      opcode::AMO => amo(instruction, self),
      opcode::OP => op(instruction, self),
      opcode::LUI => lui(instruction, self),
      opcode::OP_32 => op_32(instruction, self),
      opcode::OP_FP => op_fp(instruction, self),
      opcode::BRANCH => branch(instruction, self),
      opcode::JALR => jalr(instruction, self),
      opcode::JAL => jal(instruction, self),
      opcode::SYSTEM => system(instruction, self).await,
      _ => {
        self.perf.end_cpu_time();
        Err(Exception::IllegalInstruction(inst))
      }
    }
  }
}

/// Chapter 34. RV32/64G Instruction Set Listings;
/// Table 70. RISC-V base opcode map.
#[allow(clippy::unusual_byte_groupings)]
pub mod opcode {
  pub const LOAD: u64 = 0b00_000_11;
  pub const LOAD_FP: u64 = 0b00_001_11;
  pub const MISC_MEM: u64 = 0b00_011_11;
  pub const OP_IMM: u64 = 0b00_100_11;
  pub const AUIPC: u64 = 0b00_101_11;
  pub const OP_IMM_32: u64 = 0b00_110_11;

  pub const STORE: u64 = 0b01_000_11;
  pub const STORE_FP: u64 = 0b01_001_11;
  pub const AMO: u64 = 0b01_011_11;
  pub const OP: u64 = 0b01_100_11;
  pub const LUI: u64 = 0b01_101_11;
  pub const OP_32: u64 = 0b01_110_11;

  pub const MADD: u64 = 0b10_000_11;
  pub const MSUB: u64 = 0b10_001_11;
  pub const NMSUB: u64 = 0b10_010_11;
  pub const NMADD: u64 = 0b10_011_11;
  pub const OP_FP: u64 = 0b10_100_11;
  pub const OP_V: u64 = 0b10_101_11;

  pub const BRANCH: u64 = 0b11_000_11;
  pub const JALR: u64 = 0b11_001_11;
  pub const JAL: u64 = 0b11_011_11;
  pub const SYSTEM: u64 = 0b11_100_11;
  pub const OP_VE: u64 = 0b11_101_11;
}
