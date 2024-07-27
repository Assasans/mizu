use crate::cpu::{Cpu, Instruction};
use crate::exception::Exception;

#[inline(always)]
pub fn lui(inst: Instruction, cpu: &mut Cpu) -> Result<u64, Exception> {
  // lui
  cpu.regs[inst.rd()] = (*inst & 0xfffff000) as i32 as i64 as u64;
  cpu.perf.end_cpu_time();
  cpu.update_pc()
}
