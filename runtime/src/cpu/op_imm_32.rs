use crate::cpu::{Cpu, Instruction};
use crate::exception::Exception;

#[inline(always)]
pub fn opp_imm_32(inst: Instruction, cpu: &mut Cpu) -> Result<u64, Exception> {
  let imm = ((*inst as i32 as i64) >> 20) as u64;
  // "SLLIW, SRLIW, and SRAIW encodings with imm[5] ̸= 0 are reserved."
  let shamt = (imm & 0x1f) as u32;
  match inst.funct3() {
    0x0 => {
      // addiw
      cpu.regs[inst.rd()] = cpu.regs[inst.rs1()].wrapping_add(imm) as i32 as i64 as u64;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    0x1 => {
      // slliw
      cpu.regs[inst.rd()] = cpu.regs[inst.rs1()].wrapping_shl(shamt) as i32 as i64 as u64;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    0x5 => {
      match inst.funct7() {
        0x00 => {
          // srliw
          cpu.regs[inst.rd()] = (cpu.regs[inst.rs1()] as u32).wrapping_shr(shamt) as i32 as i64 as u64;
          cpu.perf.end_cpu_time();
          cpu.update_pc()
        }
        0x20 => {
          // sraiw
          cpu.regs[inst.rd()] = (cpu.regs[inst.rs1()] as i32).wrapping_shr(shamt) as i64 as u64;
          cpu.perf.end_cpu_time();
          cpu.update_pc()
        }
        _ => {
          cpu.perf.end_cpu_time();
          Err(Exception::IllegalInstruction(*inst))
        }
      }
    }
    _ => {
      cpu.perf.end_cpu_time();
      Err(Exception::IllegalInstruction(*inst))
    }
  }
}
