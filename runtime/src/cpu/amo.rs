use crate::cpu::{Cpu, Instruction};
use crate::exception::Exception;

#[inline(always)]
pub fn amo(inst: Instruction, cpu: &mut Cpu) -> Result<u64, Exception> {
  // RV64A: "A" standard extension for atomic instructions
  let funct5 = (inst.funct7() & 0b1111100) >> 2;
  let _aq = (inst.funct7() & 0b0000010) >> 1; // acquire access
  let _rl = inst.funct7() & 0b0000001; // release access
  match (inst.funct3(), funct5) {
    (0x2, 0x00) => {
      // amoadd.w
      let t = cpu.load(cpu.regs[inst.rs1()], 32)?;
      cpu.store(cpu.regs[inst.rs1()], 32, t.wrapping_add(cpu.regs[inst.rs2()]))?;
      cpu.regs[inst.rd()] = t;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x3, 0x00) => {
      // amoadd.d
      let t = cpu.load(cpu.regs[inst.rs1()], 64)?;
      cpu.store(cpu.regs[inst.rs1()], 64, t.wrapping_add(cpu.regs[inst.rs2()]))?;
      cpu.regs[inst.rd()] = t;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x2, 0x01) => {
      // amoswap.w
      let t = cpu.load(cpu.regs[inst.rs1()], 32)?;
      cpu.store(cpu.regs[inst.rs1()], 32, cpu.regs[inst.rs2()])?;
      cpu.regs[inst.rd()] = t;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x3, 0x01) => {
      // amoswap.d
      let t = cpu.load(cpu.regs[inst.rs1()], 64)?;
      cpu.store(cpu.regs[inst.rs1()], 64, cpu.regs[inst.rs2()])?;
      cpu.regs[inst.rd()] = t;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x3, 0x04) => {
      // amoxor.d
      let t = cpu.load(cpu.regs[inst.rs1()], 64)?;
      cpu.store(cpu.regs[inst.rs1()], 64, t ^ cpu.regs[inst.rs2()])?;
      cpu.regs[inst.rd()] = t;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x3, 0x08) => {
      // amoor.d
      let t = cpu.load(cpu.regs[inst.rs1()], 64)?;
      cpu.store(cpu.regs[inst.rs1()], 64, t | cpu.regs[inst.rs2()])?;
      cpu.regs[inst.rd()] = t;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x3, 0x12) => {
      // amoand.d
      let t = cpu.load(cpu.regs[inst.rs1()], 64)?;
      cpu.store(cpu.regs[inst.rs1()], 64, t & cpu.regs[inst.rs2()])?;
      cpu.regs[inst.rd()] = t;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x3, 0x10) => {
      // amomin.d
      let t = cpu.load(cpu.regs[inst.rs1()], 64)?;
      cpu.store(cpu.regs[inst.rs1()], 64, t.min(cpu.regs[inst.rs2()]))?;
      cpu.regs[inst.rd()] = t;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x3, 0x14) => {
      // amomax.d
      let t = cpu.load(cpu.regs[inst.rs1()], 64)?;
      cpu.store(cpu.regs[inst.rs1()], 64, t.max(cpu.regs[inst.rs2()]))?;
      cpu.regs[inst.rd()] = t;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    _ => {
      cpu.perf.end_cpu_time();
      Err(Exception::IllegalInstruction(*inst))
    }
  }
}
