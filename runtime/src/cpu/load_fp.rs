use tracing::info;

use crate::cpu::{Cpu, Instruction};
use crate::exception::Exception;

#[inline(always)]
pub fn load_fp(inst: Instruction, cpu: &mut Cpu) -> Result<u64, Exception> {
  match inst.funct3() {
    0x3 => {
      // fld
      let imm = ((*inst as i32 as i64) >> 20) as u64;
      let base_addr = cpu.regs[inst.rd()];
      let addr = base_addr + imm;

      let value = f64::from_bits(cpu.load(addr, 64).unwrap());
      info!("fld {},{},{imm}: 0x{base_addr:#08x} + {imm} (0x{addr:#08x}) get {value}", inst.rd(), inst.rs1());

      cpu.fp_regs[inst.rd()] = value;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    _ => {
      cpu.perf.end_cpu_time();
      Err(Exception::IllegalInstruction(*inst))
    }
  }
}
