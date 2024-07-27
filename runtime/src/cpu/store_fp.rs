use tracing::info;

use crate::cpu::{Cpu, Instruction};
use crate::exception::Exception;

#[inline(always)]
pub fn store_fp(inst: Instruction, cpu: &mut Cpu) -> Result<u64, Exception> {
  match inst.funct3() {
    0x3 => {
      // fsd
      let imm = ((((*inst as i32 as i64) >> 20) as u64) & 0x7f0) | (*inst >> 7) & 0x1f;
      let base_addr = cpu.regs[inst.rs1()];
      let addr = base_addr + imm;
      let value = cpu.fp_regs[inst.rs2()];

      info!(
        "fsd {},{imm}({}): 0x{base_addr:#08x} + {imm} (0x{addr:#08x}) set {value}",
        inst.rs2(),
        inst.rs1()
      );
      cpu.store(addr, 64, value.to_bits()).unwrap();
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    _ => {
      cpu.perf.end_cpu_time();
      Err(Exception::IllegalInstruction(*inst))
    }
  }
}
