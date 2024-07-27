use crate::cpu::{Cpu, Instruction};
use crate::exception::Exception;

#[inline(always)]
pub fn load(inst: Instruction, cpu: &mut Cpu) -> Result<u64, Exception> {
  // imm[11:0] = inst[31:20]
  let imm = ((*inst as i32 as i64) >> 20) as u64;
  let addr = cpu.regs[inst.rs1()].wrapping_add(imm);
  match inst.funct3() {
    0x0 => {
      // lb
      let val = cpu.load(addr, 8)?;
      cpu.regs[inst.rd()] = val as i8 as i64 as u64;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    0x1 => {
      // lh
      let val = cpu.load(addr, 16)?;
      cpu.regs[inst.rd()] = val as i16 as i64 as u64;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    0x2 => {
      // lw
      let val = cpu.load(addr, 32)?;
      cpu.regs[inst.rd()] = val as i32 as i64 as u64;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    0x3 => {
      // ld
      let val = cpu.load(addr, 64)?;
      cpu.regs[inst.rd()] = val;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    0x4 => {
      // lbu
      let val = cpu.load(addr, 8)?;
      cpu.regs[inst.rd()] = val;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    0x5 => {
      // lhu
      let val = cpu.load(addr, 16)?;
      cpu.regs[inst.rd()] = val;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    0x6 => {
      // lwu
      let val = cpu.load(addr, 32)?;
      cpu.regs[inst.rd()] = val;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    _ => {
      cpu.perf.end_cpu_time();
      Err(Exception::IllegalInstruction(*inst))
    }
  }
}
