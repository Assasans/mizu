use crate::cpu::{Cpu, Instruction};
use crate::exception::Exception;

#[inline(always)]
pub fn branch(inst: Instruction, cpu: &mut Cpu) -> Result<u64, Exception> {
  // imm[12|10:5|4:1|11] = inst[31|30:25|11:8|7]
  let imm = (((*inst & 0x80000000) as i32 as i64 >> 19) as u64)
    | ((*inst & 0x80) << 4) // imm[11]
    | ((*inst >> 20) & 0x7e0) // imm[10:5]
    | ((*inst >> 7) & 0x1e); // imm[4:1]

  match inst.funct3() {
    0x0 => {
      // beq
      if cpu.regs[inst.rs1()] == cpu.regs[inst.rs2()] {
        cpu.perf.end_cpu_time();
        return Ok(cpu.pc.wrapping_add(imm));
      }
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    0x1 => {
      // bne
      if cpu.regs[inst.rs1()] != cpu.regs[inst.rs2()] {
        cpu.perf.end_cpu_time();
        return Ok(cpu.pc.wrapping_add(imm));
      }
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    0x4 => {
      // blt
      if (cpu.regs[inst.rs1()] as i64) < (cpu.regs[inst.rs2()] as i64) {
        cpu.perf.end_cpu_time();
        return Ok(cpu.pc.wrapping_add(imm));
      }
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    0x5 => {
      // bge
      if (cpu.regs[inst.rs1()] as i64) >= (cpu.regs[inst.rs2()] as i64) {
        cpu.perf.end_cpu_time();
        return Ok(cpu.pc.wrapping_add(imm));
      }
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    0x6 => {
      // bltu
      if cpu.regs[inst.rs1()] < cpu.regs[inst.rs2()] {
        cpu.perf.end_cpu_time();
        return Ok(cpu.pc.wrapping_add(imm));
      }
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    0x7 => {
      // bgeu
      if cpu.regs[inst.rs1()] >= cpu.regs[inst.rs2()] {
        cpu.perf.end_cpu_time();
        return Ok(cpu.pc.wrapping_add(imm));
      }
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    _ => {
      cpu.perf.end_cpu_time();
      Err(Exception::IllegalInstruction(*inst))
    }
  }
}
