use crate::cpu::{Cpu, Instruction};
use crate::exception::Exception;

#[inline(always)]
pub fn auipc(inst: Instruction, cpu: &mut Cpu) -> Result<u64, Exception> {
  // auipc
  let imm = (*inst & 0xfffff000) as i32 as i64 as u64;
  cpu.regs[inst.rd()] = cpu.pc.wrapping_add(imm);
  cpu.perf.end_cpu_time();
  cpu.update_pc()
}
