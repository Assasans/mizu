use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use async_trait::async_trait;
use tracing::{debug, trace};
use crate::bus::Bus;
use crate::csr::Csr;
use crate::exception::Exception;
use crate::param::{DRAM_BASE, DRAM_SIZE};

#[async_trait]
pub trait InterruptHandler {
  async fn handle(&self, regs: &mut [u64; 32], bus: &mut Bus);
}

pub struct Cpu {
  pub regs: [u64; 32],
  pub pc: u64,
  pub bus: Bus,
  /// Control and status registers. RISC-V ISA sets aside a 12-bit encoding space (csr[11:0]) for
  /// up to 4096 CSRs.
  pub csr: Csr,
  pub ivt: HashMap<u64, Box<dyn InterruptHandler + Send>>,
}

impl Cpu {
  pub fn new(code: Vec<u8>) -> Self {
    let mut registers = [0; 32];

    // Set the register x2 with the size of a memory when a CPU is instantiated.
    registers[2] = DRAM_BASE + 0x1000;
    debug!("initialized sp=0x{:x}", registers[2]);

    let pc = DRAM_BASE;
    let bus = Bus::new(code);
    let csr = Csr::new();
    let ivt = HashMap::new();

    Cpu {
      regs: registers,
      pc,
      bus,
      csr,
      ivt,
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
    trace!("fetching instruction...");
    self.bus.load(self.pc, 32)
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

  #[inline]
  pub fn update_pc(&mut self) -> Result<u64, Exception> {
    return Ok(self.pc + 4);
  }

  pub async fn execute(&mut self, inst: u64) -> Result<u64, Exception> {
    let opcode = inst & 0x0000007f;
    let rd = ((inst & 0x00000f80) >> 7) as usize;
    let rs1 = ((inst & 0x000f8000) >> 15) as usize;
    let rs2 = ((inst & 0x01f00000) >> 20) as usize;
    let funct3 = (inst & 0x00007000) >> 12;
    let funct7 = (inst & 0xfe000000) >> 25;

    // Emulate that register x0 is hardwired with all bits equal to 0.
    self.regs[0] = 0;

    trace!("pc=0x{:x} opcode=0b{opcode:07b} ({opcode:x}) rd=0b{rd:05b} rs1=0b{rs1:05b} rs2=0b{rs2:05b} funct3=0b{funct3:03b} funct7=0b{funct7:03b}", self.pc);
    trace!("sp=0x{:x}", self.regs[2]);

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
            return self.update_pc();
          }
          0x1 => {
            // lh
            let val = self.load(addr, 16)?;
            self.regs[rd] = val as i16 as i64 as u64;
            return self.update_pc();
          }
          0x2 => {
            // lw
            let val = self.load(addr, 32)?;
            self.regs[rd] = val as i32 as i64 as u64;
            return self.update_pc();
          }
          0x3 => {
            // ld
            let val = self.load(addr, 64)?;
            self.regs[rd] = val;
            return self.update_pc();
          }
          0x4 => {
            // lbu
            let val = self.load(addr, 8)?;
            self.regs[rd] = val;
            return self.update_pc();
          }
          0x5 => {
            // lhu
            let val = self.load(addr, 16)?;
            self.regs[rd] = val;
            return self.update_pc();
          }
          0x6 => {
            // lwu
            let val = self.load(addr, 32)?;
            self.regs[rd] = val;
            return self.update_pc();
          }
          _ => Err(Exception::IllegalInstruction(inst)),
        }
      }
      0x13 => {
        // imm[11:0] = inst[31:20]
        let imm = ((inst & 0xfff00000) as i32 as i64 >> 20) as u64;
        // "The shift amount is encoded in the lower 6 bits of the I-immediate field for RV64I."
        let shamt = (imm & 0x3f) as u32;
        match funct3 {
          0x0 => {
            // addi
            debug!("addi {} - {}", self.regs[rs1], imm);
            self.regs[rd] = self.regs[rs1].wrapping_add(imm);
            return self.update_pc();
          }
          0x1 => {
            // slli
            self.regs[rd] = self.regs[rs1] << shamt;
            return self.update_pc();
          }
          0x2 => {
            // slti
            self.regs[rd] = if (self.regs[rs1] as i64) < (imm as i64) { 1 } else { 0 };
            return self.update_pc();
          }
          0x3 => {
            // sltiu
            self.regs[rd] = if self.regs[rs1] < imm { 1 } else { 0 };
            return self.update_pc();
          }
          0x4 => {
            // xori
            self.regs[rd] = self.regs[rs1] ^ imm;
            return self.update_pc();
          }
          0x5 => {
            match funct7 >> 1 {
              // srli
              0x00 => {
                self.regs[rd] = self.regs[rs1].wrapping_shr(shamt);
                return self.update_pc();
              }
              // srai
              0x10 => {
                self.regs[rd] = (self.regs[rs1] as i64).wrapping_shr(shamt) as u64;
                return self.update_pc();
              }
              _ => Err(Exception::IllegalInstruction(inst)),
            }
          }
          0x6 => {
            self.regs[rd] = self.regs[rs1] | imm;
            return self.update_pc();
          } // ori
          0x7 => {
            self.regs[rd] = self.regs[rs1] & imm; // andi
            return self.update_pc();
          }
          _ => Err(Exception::IllegalInstruction(inst)),
        }
      }
      0x17 => {
        // auipc
        let imm = (inst & 0xfffff000) as i32 as i64 as u64;
        self.regs[rd] = self.pc.wrapping_add(imm);
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
            return self.update_pc();
          }
          0x1 => {
            // slliw
            self.regs[rd] = self.regs[rs1].wrapping_shl(shamt) as i32 as i64 as u64;
            return self.update_pc();
          }
          0x5 => {
            match funct7 {
              0x00 => {
                // srliw
                self.regs[rd] = (self.regs[rs1] as u32).wrapping_shr(shamt) as i32
                  as i64 as u64;
                return self.update_pc();
              }
              0x20 => {
                // sraiw
                self.regs[rd] =
                  (self.regs[rs1] as i32).wrapping_shr(shamt) as i64 as u64;
                return self.update_pc();
              }
              _ => Err(Exception::IllegalInstruction(inst)),
            }
          }
          _ => Err(Exception::IllegalInstruction(inst)),
        }
      }
      0x23 => {
        // imm[11:5|4:0] = inst[31:25|11:7]
        let imm = (((inst & 0xfe000000) as i32 as i64 >> 20) as u64) | ((inst >> 7) & 0x1f);
        let addr = self.regs[rs1].wrapping_add(imm);
        match funct3 {
          0x0 => {
            debug!("store 8 bits at 0x{addr:x} (0x{:x} [x{rs1}] + 0x{:x})", self.regs[rs1], imm);
            self.store(addr, 8, self.regs[rs2])?;
            self.update_pc()
          } // sb
          0x1 => {
            self.store(addr, 16, self.regs[rs2])?;
            self.update_pc()
          } // sh
          0x2 => {
            self.store(addr, 32, self.regs[rs2])?;
            self.update_pc()
          } // sw
          0x3 => {
            self.store(addr, 64, self.regs[rs2])?;
            self.update_pc()
          } // sd
          _ => unreachable!(),
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
            return self.update_pc();
          }
          (0x0, 0x01) => {
            // mul
            self.regs[rd] = self.regs[rs1].wrapping_mul(self.regs[rs2]);
            return self.update_pc();
          }
          (0x0, 0x20) => {
            // sub
            self.regs[rd] = self.regs[rs1].wrapping_sub(self.regs[rs2]);
            return self.update_pc();
          }
          (0x1, 0x00) => {
            // sll
            self.regs[rd] = self.regs[rs1].wrapping_shl(shamt);
            return self.update_pc();
          }
          (0x2, 0x00) => {
            // slt
            self.regs[rd] = if (self.regs[rs1] as i64) < (self.regs[rs2] as i64) { 1 } else { 0 };
            return self.update_pc();
          }
          (0x3, 0x00) => {
            // sltu
            self.regs[rd] = if self.regs[rs1] < self.regs[rs2] { 1 } else { 0 };
            return self.update_pc();
          }
          (0x4, 0x00) => {
            // xor
            self.regs[rd] = self.regs[rs1] ^ self.regs[rs2];
            return self.update_pc();
          }
          (0x5, 0x00) => {
            // srl
            self.regs[rd] = self.regs[rs1].wrapping_shr(shamt);
            return self.update_pc();
          }
          (0x5, 0x20) => {
            // sra
            self.regs[rd] = (self.regs[rs1] as i64).wrapping_shr(shamt) as u64;
            return self.update_pc();
          }
          (0x6, 0x00) => {
            // or
            self.regs[rd] = self.regs[rs1] | self.regs[rs2];
            return self.update_pc();
          }
          (0x7, 0x00) => {
            // and
            self.regs[rd] = self.regs[rs1] & self.regs[rs2];
            return self.update_pc();
          }
          _ => Err(Exception::IllegalInstruction(inst)),
        }
      }
      0x37 => {
        // lui
        self.regs[rd] = (inst & 0xfffff000) as i32 as i64 as u64;
        return self.update_pc();
      }
      0x3b => {
        // "The shift amount is given by rs2[4:0]."
        let shamt = (self.regs[rs2] & 0x1f) as u32;
        match (funct3, funct7) {
          (0x0, 0x00) => {
            // addw
            self.regs[rd] =
              self.regs[rs1].wrapping_add(self.regs[rs2]) as i32 as i64 as u64;
            return self.update_pc();
          }
          (0x0, 0x20) => {
            // subw
            self.regs[rd] =
              ((self.regs[rs1].wrapping_sub(self.regs[rs2])) as i32) as u64;
            return self.update_pc();
          }
          (0x1, 0x00) => {
            // sllw
            self.regs[rd] = (self.regs[rs1] as u32).wrapping_shl(shamt) as i32 as u64;
            return self.update_pc();
          }
          (0x5, 0x00) => {
            // srlw
            self.regs[rd] = (self.regs[rs1] as u32).wrapping_shr(shamt) as i32 as u64;
            return self.update_pc();
          }
          (0x5, 0x20) => {
            // sraw
            self.regs[rd] = ((self.regs[rs1] as i32) >> (shamt as i32)) as u64;
            return self.update_pc();
          }
          _ => Err(Exception::IllegalInstruction(inst)),
        }
      }
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
              return Ok(self.pc.wrapping_add(imm));
            }
            return self.update_pc();
          }
          0x1 => {
            // bne
            if self.regs[rs1] != self.regs[rs2] {
              return Ok(self.pc.wrapping_add(imm));
            }
            return self.update_pc();
          }
          0x4 => {
            // blt
            if (self.regs[rs1] as i64) < (self.regs[rs2] as i64) {
              return Ok(self.pc.wrapping_add(imm));
            }
            return self.update_pc();
          }
          0x5 => {
            // bge
            if (self.regs[rs1] as i64) >= (self.regs[rs2] as i64) {
              return Ok(self.pc.wrapping_add(imm));
            }
            return self.update_pc();
          }
          0x6 => {
            // bltu
            if self.regs[rs1] < self.regs[rs2] {
              return Ok(self.pc.wrapping_add(imm));
            }
            return self.update_pc();
          }
          0x7 => {
            // bgeu
            if self.regs[rs1] >= self.regs[rs2] {
              return Ok(self.pc.wrapping_add(imm));
            }
            return self.update_pc();
          }
          _ => Err(Exception::IllegalInstruction(inst)),
        }
      }
      0x67 => {
        // jalr
        let t = self.pc + 4;

        let imm = ((((inst & 0xfff00000) as i32) as i64) >> 20) as u64;
        let new_pc = (self.regs[rs1].wrapping_add(imm)) & !1;

        self.regs[rd] = t;
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

        return Ok(self.pc.wrapping_add(imm));
      }
      0x73 => {
        let csr_addr = ((inst & 0xfff00000) >> 20) as usize;
        match funct3 {
          0x0 => {
            // ecall
            dbg!(self.dump_registers());
            let num = self.regs[17];
            debug!("executing ecall {}", num);
            if let Some(handler) = self.ivt.get(&num) {
              handler.handle(&mut self.regs, &mut self.bus).await;
            } else {
              return Err(Exception::Breakpoint(num));
            }
            return self.update_pc();
          }
          0x1 => {
            // csrrw
            let t = self.csr.load(csr_addr);
            self.csr.store(csr_addr, self.regs[rs1]);
            self.regs[rd] = t;
            return self.update_pc();
          }
          0x2 => {
            // csrrs
            let t = self.csr.load(csr_addr);
            self.csr.store(csr_addr, t | self.regs[rs1]);
            self.regs[rd] = t;
            return self.update_pc();
          }
          0x3 => {
            // csrrc
            let t = self.csr.load(csr_addr);
            self.csr.store(csr_addr, t & (!self.regs[rs1]));
            self.regs[rd] = t;
            return self.update_pc();
          }
          0x5 => {
            // csrrwi
            let zimm = rs1 as u64;
            self.regs[rd] = self.csr.load(csr_addr);
            self.csr.store(csr_addr, zimm);
            return self.update_pc();
          }
          0x6 => {
            // csrrsi
            let zimm = rs1 as u64;
            let t = self.csr.load(csr_addr);
            self.csr.store(csr_addr, t | zimm);
            self.regs[rd] = t;
            return self.update_pc();
          }
          0x7 => {
            // csrrci
            let zimm = rs1 as u64;
            let t = self.csr.load(csr_addr);
            self.csr.store(csr_addr, t & (!zimm));
            self.regs[rd] = t;
            return self.update_pc();
          }
          _ => Err(Exception::IllegalInstruction(inst)),
        }
      }
      _ => Err(Exception::IllegalInstruction(inst)),
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
