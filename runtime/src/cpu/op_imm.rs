use crate::cpu::{Cpu, Instruction};
use crate::exception::Exception;

#[inline(always)]
pub fn op_imm(inst: Instruction, cpu: &mut Cpu) -> Result<u64, Exception> {
  // imm[11:0] = inst[31:20]
  let imm = ((*inst & 0xfff00000) as i32 as i64 >> 20) as u64;
  // "The shift amount is encoded in the lower 6 bits of the I-immediate field for RV64I."
  let shamt = (imm & 0x3f) as u32;
  match inst.funct3() {
    0x0 => {
      // addi
      cpu.regs[inst.rd()] = cpu.regs[inst.rs1()].wrapping_add(imm);
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    0x1 => {
      // slli
      cpu.regs[inst.rd()] = cpu.regs[inst.rs1()] << shamt;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    0x2 => {
      // slti
      cpu.regs[inst.rd()] = if (cpu.regs[inst.rs1()] as i64) < (imm as i64) { 1 } else { 0 };
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    0x3 => {
      // sltiu
      cpu.regs[inst.rd()] = if cpu.regs[inst.rs1()] < imm { 1 } else { 0 };
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    0x4 => {
      // xori
      cpu.regs[inst.rd()] = cpu.regs[inst.rs1()] ^ imm;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    0x5 => {
      match inst.funct7() >> 1 {
        0x00 => {
          // srli
          cpu.regs[inst.rd()] = cpu.regs[inst.rs1()].wrapping_shr(shamt);
          cpu.perf.end_cpu_time();
          cpu.update_pc()
        }
        0x10 => {
          // srai
          cpu.regs[inst.rd()] = (cpu.regs[inst.rs1()] as i64).wrapping_shr(shamt) as u64;
          cpu.perf.end_cpu_time();
          cpu.update_pc()
        }
        _ => {
          cpu.perf.end_cpu_time();
          Err(Exception::IllegalInstruction(*inst))
        }
      }
    }
    0x6 => {
      // ori
      cpu.regs[inst.rd()] = cpu.regs[inst.rs1()] | imm;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    0x7 => {
      // andi
      cpu.regs[inst.rd()] = cpu.regs[inst.rs1()] & imm;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    _ => {
      cpu.perf.end_cpu_time();
      Err(Exception::IllegalInstruction(*inst))
    }
  }
}
