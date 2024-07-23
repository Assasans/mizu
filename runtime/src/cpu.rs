use std::collections::HashMap;
use std::future::Future;
use std::ops::{Div, Rem};
use std::pin::Pin;
use std::sync::{Arc, Weak};
use std::time::Instant;
use async_trait::async_trait;
use tracing::{debug, info, trace};
use crate::apic::Apic;
use crate::bus::Bus;
use crate::csr;
use crate::csr::{Csr, MASK_MEIP, MASK_MIE, MASK_MPIE, MASK_MPP, MASK_MPRV, MASK_MSIP, MASK_MTIP, MASK_SEIP, MASK_SIE, MASK_SPIE, MASK_SPP, MASK_SSIP, MASK_STIP, MCAUSE, MEPC, MIE, MIP, MSTATUS, MTVAL, MTVEC, SATP, SCAUSE, SEPC, SSTATUS, STVAL, STVEC};
use crate::exception::Exception;
use crate::interrupt::Interrupt;
use crate::isolate::Isolate;
use crate::param::{DRAM_BASE, DRAM_SIZE};
use crate::perf_counter::PerformanceCounter;
use crate::state_flow::StateFlow;

#[async_trait]
pub trait InterruptHandler: Send + Sync {
  async fn handle(&self, cpu: &mut Cpu);
}

pub struct Cpu {
  pub isolate: Option<Weak<Isolate>>,
  pub regs: [u64; 32],
  pub saved_regs: [u64; 32],
  pub fp_regs: [f64; 32],
  pub pc: u64,
  pub bus: Arc<Bus>,
  pub apic: Apic,
  /// Control and status registers. RISC-V ISA sets aside a 12-bit encoding space (csr[11:0]) for
  /// up to 4096 CSRs.
  pub csr: Csr,
  pub ivt: HashMap<u64, Arc<Box<dyn InterruptHandler>>>,
  pub perf: PerformanceCounter,
  pub halt: bool,
  pub wfi: StateFlow<bool>,
}

impl Cpu {
  pub fn new(bus: Arc<Bus>, isolate: Option<Weak<Isolate>>) -> Self {
    let mut registers = [0; 32];

    // Set the register x2 with the size of a memory when a CPU is instantiated.
    registers[2] = DRAM_BASE + 0x9000;
    debug!("initialized sp=0x{:x}", registers[2]);

    // TODO(Assasans): Wtf
    let start_time = Box::leak(Box::new(Instant::now()));
    let time = || Instant::now() - *start_time;

    let pc = DRAM_BASE;

    let mut csr = Csr::new(Box::new(time));
    csr.store(csr::machine::POWERSTATE, 1);

    let apic = Apic::new();
    let ivt = HashMap::new();
    let perf = PerformanceCounter::new();

    Cpu {
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
      wfi: StateFlow::new(false)
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

  pub fn dump_registers(&self) -> String {
    let mut output = String::new();
    let abi = [
      "zero", " ra ", " sp ", " gp ", " tp ", " t0 ", " t1 ", " t2 ", " s0 ", " s1 ", " a0 ",
      " a1 ", " a2 ", " a3 ", " a4 ", " a5 ", " a6 ", " a7 ", " s2 ", " s3 ", " s4 ", " s5 ",
      " s6 ", " s7 ", " s8 ", " s9 ", " s10", " s11", " t3 ", " t4 ", " t5 ", " t6 ",
    ];
    for i in (0..32).step_by(4) {
      output = format!(
        "{}\n{}",
        output,
        format!(
          "x{:02}({})={:>#18x} x{:02}({})={:>#18x} x{:02}({})={:>#18x} x{:02}({})={:>#18x}",
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
        )
      );
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
    match tvec_mode { // Direct
      0 => self.pc = tvec_base,
      1 => self.pc = tvec_base + (cause << 2),
      _ => unreachable!(),
    };
    debug!("interrupt handler at 0x{:x}, base: 0x{:x}, mode: {}, cause offset: 0x{:x}, pc: 0x{:x}", self.pc, tvec_base, tvec_mode, cause << 2, pc);
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
    return Ok(self.pc + 4);
  }

  pub async fn execute(&mut self, inst: u64) -> Result<u64, Exception> {
    self.perf.start_cpu_time();
    let opcode = inst & 0x0000007f;
    let rd = ((inst & 0x00000f80) >> 7) as usize;
    let rs1 = ((inst & 0x000f8000) >> 15) as usize;
    let rs2 = ((inst & 0x01f00000) >> 20) as usize;
    let funct3 = (inst & 0x00007000) >> 12;
    let funct7 = (inst & 0xfe000000) >> 25;

    // Emulate that register x0 is hardwired with all bits equal to 0.
    self.regs[0] = 0;

    trace!("pc=0x{:x} ra=0x{:x} sp=0x{:x} opcode=0b{opcode:07b} ({opcode:x}) rd=0b{rd:05b} rs1=0b{rs1:05b} rs2=0b{rs2:05b} funct3=0b{funct3:03b} funct7=0b{funct7:03b}", self.pc, self.regs[1], self.regs[2]);

    // let opcode = Opcode::from(opcode);
    // trace!("executing opcode {:?}", opcode);

    match opcode {
      0x03 => {
        // imm[11:0] = inst[31:20]
        let imm = ((inst as i32 as i64) >> 20) as u64;
        let addr = self.regs[rs1].wrapping_add(imm);
        match funct3 {
          0x0 => {
            // lb
            let val = self.load(addr, 8)?;
            self.regs[rd] = val as i8 as i64 as u64;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          0x1 => {
            // lh
            let val = self.load(addr, 16)?;
            self.regs[rd] = val as i16 as i64 as u64;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          0x2 => {
            // lw
            let val = self.load(addr, 32)?;
            self.regs[rd] = val as i32 as i64 as u64;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          0x3 => {
            // ld
            let val = self.load(addr, 64)?;
            self.regs[rd] = val;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          0x4 => {
            // lbu
            let val = self.load(addr, 8)?;
            self.regs[rd] = val;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          0x5 => {
            // lhu
            let val = self.load(addr, 16)?;
            self.regs[rd] = val;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          0x6 => {
            // lwu
            let val = self.load(addr, 32)?;
            self.regs[rd] = val;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          _ => {
            self.perf.end_cpu_time();
            Err(Exception::IllegalInstruction(inst))
          }
        }
      }
      0x07 => {
        match funct3 {
          0x3 => {
            // fld
            let imm = ((inst as i32 as i64) >> 20) as u64;
            let base_addr = self.regs[rs1];
            let addr = base_addr + imm;

            let value = f64::from_bits(self.load(addr, 64).unwrap());
            info!("fld {rd},{rs1},{imm}: 0x{base_addr:#08x} + {imm} (0x{addr:#08x}) get {value}");

            self.fp_regs[rd] = value;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          _ => {
            self.perf.end_cpu_time();
            Err(Exception::IllegalInstruction(inst))
          }
        }
      },
      0x13 => {
        // imm[11:0] = inst[31:20]
        let imm = ((inst & 0xfff00000) as i32 as i64 >> 20) as u64;
        // "The shift amount is encoded in the lower 6 bits of the I-immediate field for RV64I."
        let shamt = (imm & 0x3f) as u32;
        match funct3 {
          0x0 => {
            // addi
            self.regs[rd] = self.regs[rs1].wrapping_add(imm);
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          0x1 => {
            // slli
            self.regs[rd] = self.regs[rs1] << shamt;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          0x2 => {
            // slti
            self.regs[rd] = if (self.regs[rs1] as i64) < (imm as i64) { 1 } else { 0 };
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          0x3 => {
            // sltiu
            self.regs[rd] = if self.regs[rs1] < imm { 1 } else { 0 };
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          0x4 => {
            // xori
            self.regs[rd] = self.regs[rs1] ^ imm;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          0x5 => {
            match funct7 >> 1 {
              // srli
              0x00 => {
                self.regs[rd] = self.regs[rs1].wrapping_shr(shamt);
                self.perf.end_cpu_time();
                return self.update_pc();
              }
              // srai
              0x10 => {
                self.regs[rd] = (self.regs[rs1] as i64).wrapping_shr(shamt) as u64;
                self.perf.end_cpu_time();
                return self.update_pc();
              }
              _ => {
                self.perf.end_cpu_time();
                Err(Exception::IllegalInstruction(inst))
              }
            }
          }
          0x6 => {
            self.regs[rd] = self.regs[rs1] | imm;
            self.perf.end_cpu_time();
            return self.update_pc();
          } // ori
          0x7 => {
            self.regs[rd] = self.regs[rs1] & imm; // andi
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          _ => {
            self.perf.end_cpu_time();
            Err(Exception::IllegalInstruction(inst))
          }
        }
      }
      0x17 => {
        // auipc
        let imm = (inst & 0xfffff000) as i32 as i64 as u64;
        self.regs[rd] = self.pc.wrapping_add(imm);
        self.perf.end_cpu_time();
        return self.update_pc();
      }
      0x1b => {
        let imm = ((inst as i32 as i64) >> 20) as u64;
        // "SLLIW, SRLIW, and SRAIW encodings with imm[5] ̸= 0 are reserved."
        let shamt = (imm & 0x1f) as u32;
        match funct3 {
          0x0 => {
            // addiw
            self.regs[rd] = self.regs[rs1].wrapping_add(imm) as i32 as i64 as u64;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          0x1 => {
            // slliw
            self.regs[rd] = self.regs[rs1].wrapping_shl(shamt) as i32 as i64 as u64;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          0x5 => {
            match funct7 {
              0x00 => {
                // srliw
                self.regs[rd] = (self.regs[rs1] as u32).wrapping_shr(shamt) as i32 as i64 as u64;
                self.perf.end_cpu_time();
                return self.update_pc();
              }
              0x20 => {
                // sraiw
                self.regs[rd] = (self.regs[rs1] as i32).wrapping_shr(shamt) as i64 as u64;
                self.perf.end_cpu_time();
                return self.update_pc();
              }
              _ => {
                self.perf.end_cpu_time();
                Err(Exception::IllegalInstruction(inst))
              },
            }
          }
          _ => {
            self.perf.end_cpu_time();
            Err(Exception::IllegalInstruction(inst))
          },
        }
      }
      0x23 => {
        // imm[11:5|4:0] = inst[31:25|11:7]
        let imm = (((inst & 0xfe000000) as i32 as i64 >> 20) as u64) | ((inst >> 7) & 0x1f);
        let addr = self.regs[rs1].wrapping_add(imm);
        match funct3 {
          0x0 => {
            self.store(addr, 8, self.regs[rs2])?;
            self.perf.end_cpu_time();
            self.update_pc()
          } // sb
          0x1 => {
            self.store(addr, 16, self.regs[rs2])?;
            self.perf.end_cpu_time();
            self.update_pc()
          } // sh
          0x2 => {
            self.store(addr, 32, self.regs[rs2])?;
            self.perf.end_cpu_time();
            self.update_pc()
          } // sw
          0x3 => {
            self.store(addr, 64, self.regs[rs2])?;
            self.perf.end_cpu_time();
            self.update_pc()
          } // sd
          _ => unreachable!(),
        }
      }
      0x27 => {
        match funct3 {
          0x3 => {
            // fsd
            let imm = ((((inst as i32 as i64) >> 20) as u64) & 0x7f0) | (inst >> 7) & 0x1f;
            let base_addr = self.regs[rs1];
            let addr = base_addr + imm;
            let value = self.fp_regs[rs2];

            info!("fsd {rs2},{imm}({rs1}): 0x{base_addr:#08x} + {imm} (0x{addr:#08x}) set {value}");
            self.store(addr, 64, value.to_bits()).unwrap();
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          _ => {
            self.perf.end_cpu_time();
            Err(Exception::IllegalInstruction(inst))
          }
        }
      },
      0x2f => {
        // RV64A: "A" standard extension for atomic instructions
        let funct5 = (funct7 & 0b1111100) >> 2;
        let _aq = (funct7 & 0b0000010) >> 1; // acquire access
        let _rl = funct7 & 0b0000001; // release access
        match (funct3, funct5) {
          (0x2, 0x00) => {
            // amoadd.w
            let t = self.load(self.regs[rs1], 32)?;
            self.store(self.regs[rs1], 32, t.wrapping_add(self.regs[rs2]))?;
            self.regs[rd] = t;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x3, 0x00) => {
            // amoadd.d
            let t = self.load(self.regs[rs1], 64)?;
            self.store(self.regs[rs1], 64, t.wrapping_add(self.regs[rs2]))?;
            self.regs[rd] = t;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x2, 0x01) => {
            // amoswap.w
            let t = self.load(self.regs[rs1], 32)?;
            self.store(self.regs[rs1], 32, self.regs[rs2])?;
            self.regs[rd] = t;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x3, 0x01) => {
            // amoswap.d
            let t = self.load(self.regs[rs1], 64)?;
            self.store(self.regs[rs1], 64, self.regs[rs2])?;
            self.regs[rd] = t;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x3, 0x04) => {
            // amoxor.d
            let t = self.load(self.regs[rs1], 64)?;
            self.store(self.regs[rs1], 64, t ^ self.regs[rs2])?;
            self.regs[rd] = t;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x3, 0x08) => {
            // amoor.d
            let t = self.load(self.regs[rs1], 64)?;
            self.store(self.regs[rs1], 64, t | self.regs[rs2])?;
            self.regs[rd] = t;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x3, 0x12) => {
            // amoand.d
            let t = self.load(self.regs[rs1], 64)?;
            self.store(self.regs[rs1], 64, t & self.regs[rs2])?;
            self.regs[rd] = t;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x3, 0x10) => {
            // amomin.d
            let t = self.load(self.regs[rs1], 64)?;
            self.store(self.regs[rs1], 64, t.min(self.regs[rs2]))?;
            self.regs[rd] = t;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x3, 0x14) => {
            // amomax.d
            let t = self.load(self.regs[rs1], 64)?;
            self.store(self.regs[rs1], 64, t.max(self.regs[rs2]))?;
            self.regs[rd] = t;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          _ => {
            self.perf.end_cpu_time();
            Err(Exception::IllegalInstruction(inst))
          },
        }
      }
      0x33 => {
        // "SLL, SRL, and SRA perform logical left, logical right, and arithmetic right
        // shifts on the value in register rs1 by the shift amount held in register rs2.
        // In RV64I, only the low 6 bits of rs2 are considered for the shift amount."
        let shamt = ((self.regs[rs2] & 0x3f) as u64) as u32;
        match (funct3, funct7) {
          (0x0, 0x00) => {
            // add
            self.regs[rd] = self.regs[rs1].wrapping_add(self.regs[rs2]);
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x0, 0x01) => {
            // mul
            self.regs[rd] = self.regs[rs1].wrapping_mul(self.regs[rs2]);
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x0, 0x20) => {
            // sub
            self.regs[rd] = self.regs[rs1].wrapping_sub(self.regs[rs2]);
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x1, 0x00) => {
            // sll
            self.regs[rd] = self.regs[rs1].wrapping_shl(shamt);
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x1, 0x01) => {
            // mulh
            self.regs[rd] = ((self.regs[rs1] as i128).wrapping_mul(self.regs[rs2] as i128) >> 64) as u64;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x2, 0x00) => {
            // slt
            self.regs[rd] = if (self.regs[rs1] as i64) < (self.regs[rs2] as i64) { 1 } else { 0 };
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x3, 0x00) => {
            // sltu
            self.regs[rd] = if self.regs[rs1] < self.regs[rs2] { 1 } else { 0 };
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x3, 0x01) => {
            // mulhu
            self.regs[rd] = ((self.regs[rs1] as u128).wrapping_mul(self.regs[rs2] as u128) >> 64) as u64;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x4, 0x00) => {
            // xor
            self.regs[rd] = self.regs[rs1] ^ self.regs[rs2];
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x5, 0x00) => {
            // srl
            self.regs[rd] = self.regs[rs1].wrapping_shr(shamt);
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x5, 0x01) => {
            // divu
            self.regs[rd] = self.regs[rs1].wrapping_div(self.regs[rs2]);
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x5, 0x20) => {
            // sra
            self.regs[rd] = (self.regs[rs1] as i64).wrapping_shr(shamt) as u64;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x6, 0x00) => {
            // or
            self.regs[rd] = self.regs[rs1] | self.regs[rs2];
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x7, 0x00) => {
            // and
            self.regs[rd] = self.regs[rs1] & self.regs[rs2];
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x7, 0x01) => {
            // remu
            self.regs[rd] = self.regs[rs1].wrapping_rem(self.regs[rs2]);
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          _ => {
            self.perf.end_cpu_time();
            Err(Exception::IllegalInstruction(inst))
          },
        }
      }
      0x37 => {
        // lui
        self.regs[rd] = (inst & 0xfffff000) as i32 as i64 as u64;
        self.perf.end_cpu_time();
        return self.update_pc();
      }
      0x3b => {
        // "The shift amount is given by rs2[4:0]."
        let shamt = (self.regs[rs2] & 0x1f) as u32;
        match (funct3, funct7) {
          (0x0, 0x00) => {
            // addw
            self.regs[rd] = self.regs[rs1].wrapping_add(self.regs[rs2]) as i32 as i64 as u64;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x0, 0x1) => {
            // mulw
            self.regs[rd] = (self.regs[rs1] as i32).wrapping_mul(self.regs[rs2] as i32) as i64 as u64;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x0, 0x20) => {
            // subw
            self.regs[rd] = (self.regs[rs1].wrapping_sub(self.regs[rs2]) as i32) as u64;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x1, 0x00) => {
            // sllw
            self.regs[rd] = (self.regs[rs1] as u32).wrapping_shl(shamt) as i32 as u64;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x4, 0x1) => {
            // divw
            self.regs[rd] = (self.regs[rs1] as i32).wrapping_div(self.regs[rs2] as i32) as i64 as u64;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x5, 0x00) => {
            // srlw
            self.regs[rd] = (self.regs[rs1] as u32).wrapping_shr(shamt) as i32 as u64;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x5, 0x1) => {
            // divuw
            self.regs[rd] = (self.regs[rs1] as u32).wrapping_div(self.regs[rs2] as u32) as u64;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x5, 0x20) => {
            // sraw
            self.regs[rd] = ((self.regs[rs1] as i32) >> (shamt as i32)) as u64;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          _ => {
            self.perf.end_cpu_time();
            Err(Exception::IllegalInstruction(inst))
          },
        }
      }
      0x53 => {
        // Only signaling NaN inputs cause an Invalid Operation exception.
        // The result is 0 if either operand is NaN.
        match (funct3, funct7) {
          (_, 0x9) => {
            // fmul.d
            self.fp_regs[rd] = self.fp_regs[rs1] * self.fp_regs[rs2];
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x0, 0x51) => {
            // fle.d
            // Performs a quiet less or equal comparison between floating-point registers rs1 and rs2 and record the Boolean result in integer register rd.
            self.regs[rd] = if self.regs[rs1] <= self.regs[rs2] { 1 } else { 0 };
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x1, 0x51) => {
            // flt.d
            // Performs a quiet less comparison between floating-point registers rs1 and rs2 and record the Boolean result in integer register rd.
            self.regs[rd] = if self.regs[rs1] < self.regs[rs2] { 1 } else { 0 };
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x2, 0x51) => {
            // feq.d
            // Performs a quiet equal comparison between floating-point registers rs1 and rs2 and record the Boolean result in integer register rd.
            // Only signaling NaN inputs cause an Invalid Operation exception.
            // The result is 0 if either operand is NaN.
            self.regs[rd] = if self.regs[rs1] == self.regs[rs2] { 1 } else { 0 };
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x0, 0x71) => {
            // fmv.x.d
            self.regs[rd] = self.fp_regs[rs1].to_bits();
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          (0x0, 0x79) => {
            // fmv.d.x
            self.fp_regs[rd] = f64::from_bits(self.regs[rs1]);
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          _ => {
            self.perf.end_cpu_time();
            Err(Exception::IllegalInstruction(inst))
          }
        }
      },
      0x63 => {
        // imm[12|10:5|4:1|11] = inst[31|30:25|11:8|7]
        let imm = (((inst & 0x80000000) as i32 as i64 >> 19) as u64)
          | ((inst & 0x80) << 4) // imm[11]
          | ((inst >> 20) & 0x7e0) // imm[10:5]
          | ((inst >> 7) & 0x1e); // imm[4:1]

        match funct3 {
          0x0 => {
            // beq
            if self.regs[rs1] == self.regs[rs2] {
              self.perf.end_cpu_time();
              return Ok(self.pc.wrapping_add(imm));
            }
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          0x1 => {
            // bne
            if self.regs[rs1] != self.regs[rs2] {
              self.perf.end_cpu_time();
              return Ok(self.pc.wrapping_add(imm));
            }
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          0x4 => {
            // blt
            if (self.regs[rs1] as i64) < (self.regs[rs2] as i64) {
              self.perf.end_cpu_time();
              return Ok(self.pc.wrapping_add(imm));
            }
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          0x5 => {
            // bge
            if (self.regs[rs1] as i64) >= (self.regs[rs2] as i64) {
              self.perf.end_cpu_time();
              return Ok(self.pc.wrapping_add(imm));
            }
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          0x6 => {
            // bltu
            if self.regs[rs1] < self.regs[rs2] {
              self.perf.end_cpu_time();
              return Ok(self.pc.wrapping_add(imm));
            }
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          0x7 => {
            // bgeu
            if self.regs[rs1] >= self.regs[rs2] {
              self.perf.end_cpu_time();
              return Ok(self.pc.wrapping_add(imm));
            }
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          _ => {
            self.perf.end_cpu_time();
            Err(Exception::IllegalInstruction(inst))
          },
        }
      }
      0x67 => {
        // jalr
        let t = self.pc + 4;

        let imm = ((((inst & 0xfff00000) as i32) as i64) >> 20) as u64;
        let new_pc = (self.regs[rs1].wrapping_add(imm)) & !1;
        debug!("ret 0x{imm:x} -> 0x{new_pc:x} read from {}", rs1);

        self.regs[rd] = t;
        self.perf.end_cpu_time();
        return Ok(new_pc);
      }
      0x6f => {
        // jal
        self.regs[rd] = self.pc + 4;

        // imm[20|10:1|11|19:12] = inst[31|30:21|20|19:12]
        let imm = (((inst & 0x80000000) as i32 as i64 >> 11) as u64) // imm[20]
          | (inst & 0xff000) // imm[19:12]
          | ((inst >> 9) & 0x800) // imm[11]
          | ((inst >> 20) & 0x7fe); // imm[10:1]

        self.perf.end_cpu_time();
        return Ok(self.pc.wrapping_add(imm));
      }
      0x73 => {
        let csr_addr = ((inst & 0xfff00000) >> 20) as usize;
        match funct3 {
          0x0 => {
            match (rs2, funct7) {
              // ECALL and EBREAK cause the receiving privilege mode’s epc register to be set to the address of
              // the ECALL or EBREAK instruction itself, not the address of the following instruction.
              (0x0, 0x0) => {
                // ecall
                let num = self.regs[17];
                debug!("executing ecall {}", num);
                if let Some(handler) = self.ivt.get(&num) {
                  self.perf.end_cpu_time();
                  let handler = handler.clone();
                  // syscalls are not cpu time limited
                  handler.handle(self).await;
                  self.perf.start_cpu_time();
                } else {
                  return Err(Exception::RuntimeFault(num));
                }
                self.perf.end_cpu_time();
                return self.update_pc();
              }
              (0x1, 0x0) => {
                // ebreak
                // Makes a request of the debugger bu raising a Breakpoint exception.
                self.perf.end_cpu_time();
                return Err(Exception::Breakpoint(self.pc));
              }
              (0x2, 0x18) => {
                // mret
                if self.csr.load(MCAUSE) == 0 {
                  self.perf.end_cpu_time();
                  return Err(Exception::RuntimeFault(333));
                }

                // Restore registers
                self.regs.swap_with_slice(&mut self.saved_regs);
                // self.regs.copy_from_slice(&self.saved_regs);
                // self.saved_regs.fill(0);

                debug!("trap exit: 0x{:x} -> 0x{:x}", self.pc, self.csr.load(MEPC));
                self.pc = self.csr.load(MEPC);
                self.csr.store(MEPC, 0);
                self.csr.store(MCAUSE, 0);
                self.csr.store(MTVAL, 0);

                // TODO(Assasans): I have no idea what to do with MSTATUS
                let mut status = self.csr.load(MSTATUS);

                let ie = (status & MASK_MPIE) >> 7;
                // set MIE = MPIE
                status = (status & !MASK_MIE) | (ie << 3);
                // set MPIE = 0
                status &= !MASK_MPIE;
                self.csr.store(MSTATUS, status);

                self.perf.end_cpu_time();
                return Ok(self.pc);
                // return self.update_pc();
              }
              (0x5, 0x8) => {
                // wfi
                info!("waiting for interrupt");
                self.wfi.set(true);
                self.perf.end_cpu_time();
                return self.update_pc();
              }
              (_, 0x9) => {
                // sfence.vma
                // Do nothing.
                self.perf.end_cpu_time();
                return self.update_pc();
              }
              _ => {
                self.perf.end_cpu_time();
                Err(Exception::IllegalInstruction(inst))
              },
            }
          }
          0x1 => {
            // csrrw
            let t = self.csr.load(csr_addr);
            self.csr.store(csr_addr, self.regs[rs1]);
            self.regs[rd] = t;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          0x2 => {
            // csrrs
            let t = self.csr.load(csr_addr);
            self.csr.store(csr_addr, t | self.regs[rs1]);
            self.regs[rd] = t;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          0x3 => {
            // csrrc
            let t = self.csr.load(csr_addr);
            self.csr.store(csr_addr, t & (!self.regs[rs1]));
            self.regs[rd] = t;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          0x5 => {
            // csrrwi
            let zimm = rs1 as u64;
            self.regs[rd] = self.csr.load(csr_addr);
            self.csr.store(csr_addr, zimm);
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          0x6 => {
            // csrrsi
            let zimm = rs1 as u64;
            let t = self.csr.load(csr_addr);
            self.csr.store(csr_addr, t | zimm);
            self.regs[rd] = t;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          0x7 => {
            // csrrci
            let zimm = rs1 as u64;
            let t = self.csr.load(csr_addr);
            self.csr.store(csr_addr, t & (!zimm));
            self.regs[rd] = t;
            self.perf.end_cpu_time();
            return self.update_pc();
          }
          _ => {
            self.perf.end_cpu_time();
            Err(Exception::IllegalInstruction(inst))
          },
        }
      }
      _ => {
        self.perf.end_cpu_time();
        Err(Exception::IllegalInstruction(inst))
      },
    }
  }
}

macro_rules! define_opcodes {
  ( $( $name:ident => $value:expr ),* $(,)? ) => {
    #[non_exhaustive]
    #[derive(Debug, PartialEq, Eq)]
    pub enum Opcode {
      $($name),*,
      Unknown(u64)
    }

    impl From<u64> for Opcode {
      fn from(value: u64) -> Self {
        match value {
          $( $value => Opcode::$name, )*
          _ => Opcode::Unknown(value),
        }
      }
    }

    impl From<&Opcode> for u64 {
      fn from(value: &Opcode) -> Self {
        match value {
          $( Opcode::$name => $value, )*
          Opcode::Unknown(value) => *value,
        }
      }
    }
  };
}

define_opcodes!(
  Addi => 0x13,
  Add => 0x33,
  Ecall => 0x73
);
