use tracing::trace;

use crate::cpu::{Cpu, Instruction};
use crate::exception::Exception;

#[inline(always)]
pub fn store_fp(inst: Instruction, cpu: &mut Cpu) -> Result<u64, Exception> {
  match inst.funct3() {
    0x2 => {
      // fsw
      let imm = ((*inst >> 25) & 0x7F) | ((*inst >> 7) & 0x1F);
      let base_addr = cpu.regs[inst.rs1()];
      let addr = base_addr + imm;
      let value = cpu.fp_regs[inst.rs2()] as f32;

      trace!(
        "fsw {},{imm}({}): 0x{base_addr:#08x} + {imm} (0x{addr:#08x}) set {value}",
        inst.rs2(),
        inst.rs1()
      );
      cpu.store(addr, 32, value.to_bits() as u64).unwrap();
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    0x3 => {
      // fsd
      let imm = ((*inst >> 25) & 0x7F) | ((*inst >> 7) & 0x1F);
      let base_addr = cpu.regs[inst.rs1()];
      let addr = base_addr + imm;
      let value = cpu.fp_regs[inst.rs2()];

      trace!(
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
