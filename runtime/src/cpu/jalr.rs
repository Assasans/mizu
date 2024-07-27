use tracing::debug;

use crate::cpu::{Cpu, Instruction};
use crate::exception::Exception;

#[inline(always)]
pub fn jalr(inst: Instruction, cpu: &mut Cpu) -> Result<u64, Exception> {
  // jalr
  let t = cpu.pc + 4;

  let imm = ((((*inst & 0xfff00000) as i32) as i64) >> 20) as u64;
  let new_pc = (cpu.regs[inst.rs1()].wrapping_add(imm)) & !1;
  debug!("ret 0x{imm:x} -> 0x{new_pc:x} read from {}", inst.rs1());

  cpu.regs[inst.rd()] = t;
  cpu.perf.end_cpu_time();
  Ok(new_pc)
}
