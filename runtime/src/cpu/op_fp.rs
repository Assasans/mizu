use crate::cpu::{Cpu, Instruction};
use crate::exception::Exception;

#[inline(always)]
pub fn op_fp(inst: Instruction, cpu: &mut Cpu) -> Result<u64, Exception> {
  // Only signaling NaN inputs cause an Invalid Operation exception.
  // The result is 0 if either operand is NaN.
  match (inst.funct3(), inst.funct7()) {
    (_, 0x9) => {
      // fmul.d
      cpu.fp_regs[inst.rd()] = cpu.fp_regs[inst.rs1()] * cpu.fp_regs[inst.rs2()];
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x0, 0x51) => {
      // fle.d
      // Performs a quiet less or equal comparison between floating-point registers rs1 and rs2 and record the Boolean result in integer register rd.
      cpu.regs[inst.rd()] = if cpu.regs[inst.rs1()] <= cpu.regs[inst.rs2()] { 1 } else { 0 };
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x1, 0x51) => {
      // flt.d
      // Performs a quiet less comparison between floating-point registers rs1 and rs2 and record the Boolean result in integer register rd.
      cpu.regs[inst.rd()] = if cpu.regs[inst.rs1()] < cpu.regs[inst.rs2()] { 1 } else { 0 };
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x2, 0x51) => {
      // feq.d
      // Performs a quiet equal comparison between floating-point registers rs1 and rs2 and record the Boolean result in integer register rd.
      // Only signaling NaN inputs cause an Invalid Operation exception.
      // The result is 0 if either operand is NaN.
      cpu.regs[inst.rd()] = if cpu.regs[inst.rs1()] == cpu.regs[inst.rs2()] { 1 } else { 0 };
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x0, 0x71) => {
      // fmv.x.d
      cpu.regs[inst.rd()] = cpu.fp_regs[inst.rs1()].to_bits();
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x0, 0x79) => {
      // fmv.d.x
      cpu.fp_regs[inst.rd()] = f64::from_bits(cpu.regs[inst.rs1()]);
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    _ => {
      cpu.perf.end_cpu_time();
      Err(Exception::IllegalInstruction(*inst))
    }
  }
}
