use crate::cpu::{Cpu, Instruction};
use crate::exception::Exception;

#[inline(always)]
pub fn store(inst: Instruction, cpu: &mut Cpu) -> Result<u64, Exception> {
  // imm[11:5|4:0] = inst[31:25|11:7]
  let imm = (((*inst & 0xfe000000) as i32 as i64 >> 20) as u64) | ((*inst >> 7) & 0x1f);
  let addr = cpu.regs[inst.rs1()].wrapping_add(imm);
  match inst.funct3() {
    0x0 => {
      cpu.store(addr, 8, cpu.regs[inst.rs2()])?;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    } // sb
    0x1 => {
      cpu.store(addr, 16, cpu.regs[inst.rs2()])?;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    } // sh
    0x2 => {
      cpu.store(addr, 32, cpu.regs[inst.rs2()])?;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    } // sw
    0x3 => {
      cpu.store(addr, 64, cpu.regs[inst.rs2()])?;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    } // sd
    _ => unreachable!(),
  }
}
