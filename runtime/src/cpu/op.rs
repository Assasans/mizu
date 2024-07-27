use crate::cpu::{Cpu, Instruction};
use crate::exception::Exception;

#[inline(always)]
pub fn op(inst: Instruction, cpu: &mut Cpu) -> Result<u64, Exception> {
  // "SLL, SRL, and SRA perform logical left, logical right, and arithmetic right
  // shifts on the value in register rs1 by the shift amount held in register rs2.
  // In RV64I, only the low 6 bits of rs2 are considered for the shift amount."
  let shamt = (cpu.regs[inst.rs2()] & 0x3f) as u32;
  match (inst.funct3(), inst.funct7()) {
    (0x0, 0x00) => {
      // add
      cpu.regs[inst.rd()] = cpu.regs[inst.rs1()].wrapping_add(cpu.regs[inst.rs2()]);
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x0, 0x01) => {
      // mul
      cpu.regs[inst.rd()] = cpu.regs[inst.rs1()].wrapping_mul(cpu.regs[inst.rs2()]);
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x0, 0x20) => {
      // sub
      cpu.regs[inst.rd()] = cpu.regs[inst.rs1()].wrapping_sub(cpu.regs[inst.rs2()]);
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x1, 0x00) => {
      // sll
      cpu.regs[inst.rd()] = cpu.regs[inst.rs1()].wrapping_shl(shamt);
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x1, 0x01) => {
      // mulh
      cpu.regs[inst.rd()] = ((cpu.regs[inst.rs1()] as i128).wrapping_mul(cpu.regs[inst.rs2()] as i128) >> 64) as u64;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x2, 0x00) => {
      // slt
      cpu.regs[inst.rd()] = if (cpu.regs[inst.rs1()] as i64) < (cpu.regs[inst.rs2()] as i64) {
        1
      } else {
        0
      };
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x3, 0x00) => {
      // sltu
      cpu.regs[inst.rd()] = if cpu.regs[inst.rs1()] < cpu.regs[inst.rs2()] { 1 } else { 0 };
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x3, 0x01) => {
      // mulhu
      cpu.regs[inst.rd()] = ((cpu.regs[inst.rs1()] as u128).wrapping_mul(cpu.regs[inst.rs2()] as u128) >> 64) as u64;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x4, 0x00) => {
      // xor
      cpu.regs[inst.rd()] = cpu.regs[inst.rs1()] ^ cpu.regs[inst.rs2()];
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x5, 0x00) => {
      // srl
      cpu.regs[inst.rd()] = cpu.regs[inst.rs1()].wrapping_shr(shamt);
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x5, 0x01) => {
      // divu
      cpu.regs[inst.rd()] = cpu.regs[inst.rs1()].wrapping_div(cpu.regs[inst.rs2()]);
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x5, 0x20) => {
      // sra
      cpu.regs[inst.rd()] = (cpu.regs[inst.rs1()] as i64).wrapping_shr(shamt) as u64;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x6, 0x00) => {
      // or
      cpu.regs[inst.rd()] = cpu.regs[inst.rs1()] | cpu.regs[inst.rs2()];
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x7, 0x00) => {
      // and
      cpu.regs[inst.rd()] = cpu.regs[inst.rs1()] & cpu.regs[inst.rs2()];
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x7, 0x01) => {
      // remu
      cpu.regs[inst.rd()] = cpu.regs[inst.rs1()].wrapping_rem(cpu.regs[inst.rs2()]);
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    _ => {
      cpu.perf.end_cpu_time();
      Err(Exception::IllegalInstruction(*inst))
    }
  }
}
