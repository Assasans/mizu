use crate::cpu::{Cpu, Instruction};
use crate::exception::Exception;

#[inline(always)]
pub fn op_32(inst: Instruction, cpu: &mut Cpu) -> Result<u64, Exception> {
  // "The shift amount is given by inst.rs2()[4:0]."
  let shamt = (cpu.regs[inst.rs2()] & 0x1f) as u32;
  match (inst.funct3(), inst.funct7()) {
    (0x0, 0x00) => {
      // addw
      cpu.regs[inst.rd()] = cpu.regs[inst.rs1()].wrapping_add(cpu.regs[inst.rs2()]) as i32 as i64 as u64;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x0, 0x1) => {
      // mulw
      cpu.regs[inst.rd()] = (cpu.regs[inst.rs1()] as i32).wrapping_mul(cpu.regs[inst.rs2()] as i32) as i64 as u64;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x0, 0x20) => {
      // subw
      cpu.regs[inst.rd()] = (cpu.regs[inst.rs1()].wrapping_sub(cpu.regs[inst.rs2()]) as i32) as u64;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x1, 0x00) => {
      // sllw
      cpu.regs[inst.rd()] = (cpu.regs[inst.rs1()] as u32).wrapping_shl(shamt) as i32 as u64;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x4, 0x1) => {
      // divw
      cpu.regs[inst.rd()] = (cpu.regs[inst.rs1()] as i32).wrapping_div(cpu.regs[inst.rs2()] as i32) as i64 as u64;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x5, 0x00) => {
      // srlw
      cpu.regs[inst.rd()] = (cpu.regs[inst.rs1()] as u32).wrapping_shr(shamt) as i32 as u64;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x5, 0x1) => {
      // divuw
      cpu.regs[inst.rd()] = (cpu.regs[inst.rs1()] as u32).wrapping_div(cpu.regs[inst.rs2()] as u32) as u64;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x5, 0x20) => {
      // sraw
      cpu.regs[inst.rd()] = ((cpu.regs[inst.rs1()] as i32) >> (shamt as i32)) as u64;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    _ => {
      cpu.perf.end_cpu_time();
      Err(Exception::IllegalInstruction(*inst))
    }
  }
}
