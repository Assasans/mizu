use crate::cpu::{Cpu, Instruction};
use crate::exception::Exception;

#[inline(always)]
pub fn jal(inst: Instruction, cpu: &mut Cpu) -> Result<u64, Exception> {
  // jal
  cpu.regs[inst.rd()] = cpu.pc + 4;

  // imm[20|10:1|11|19:12] = inst[31|30:21|20|19:12]
  let imm = (((*inst & 0x80000000) as i32 as i64 >> 11) as u64) // imm[20]
    | (*inst & 0xff000) // imm[19:12]
    | ((*inst >> 9) & 0x800) // imm[11]
    | ((*inst >> 20) & 0x7fe); // imm[10:1]

  cpu.perf.end_cpu_time();
  Ok(cpu.pc.wrapping_add(imm))
}
