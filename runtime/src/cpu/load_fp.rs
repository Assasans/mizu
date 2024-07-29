use tracing::trace;

use crate::cpu::{Cpu, Instruction};
use crate::exception::Exception;

#[inline(always)]
pub fn load_fp(inst: Instruction, cpu: &mut Cpu) -> Result<u64, Exception> {
  match inst.funct3() {
    0x2 => {
      // flw
      let imm = ((*inst as i32 as i64) >> 20) as u64;
      let base_addr = cpu.regs[inst.rs1()];
      let addr = base_addr.wrapping_add(imm);

      trace!("flw {},{},{imm}: 0x{base_addr:#08x} + {imm} (0x{addr:#08x})", inst.rd(), inst.rs1());
      let value = f32::from_bits(cpu.load(addr, 32).unwrap() as u32);

      cpu.fp_regs[inst.rd()] = value as f64;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    0x3 => {
      // fld
      let imm = ((*inst as i32 as i64) >> 20) as u64;
      let base_addr = cpu.regs[inst.rs1()];
      let addr = base_addr.wrapping_add(imm);

      trace!("fld {},{},{imm}: 0x{base_addr:#08x} + {imm} (0x{addr:#08x})", inst.rd(), inst.rs1());
      let value = f64::from_bits(cpu.load(addr, 64).unwrap());

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
