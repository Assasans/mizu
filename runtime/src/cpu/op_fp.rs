use std::ops::Rem;
use tracing::trace;

use crate::cpu::{Cpu, Instruction};
use crate::exception::Exception;

#[inline(always)]
pub fn op_fp(inst: Instruction, cpu: &mut Cpu) -> Result<u64, Exception> {
  // Only signaling NaN inputs cause an Invalid Operation exception.
  // The result is 0 if either operand is NaN.
  match (inst.funct3(), inst.funct7()) {
    (_, 0x1) => {
      // fadd.d
      cpu.fp_regs[inst.rd()] = cpu.fp_regs[inst.rs1()] + cpu.fp_regs[inst.rs2()];
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (_, 0x5) => {
      // fsub.d
      cpu.fp_regs[inst.rd()] = cpu.fp_regs[inst.rs1()] - cpu.fp_regs[inst.rs2()];
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (_, 0x9) => {
      // fmul.d
      cpu.fp_regs[inst.rd()] = cpu.fp_regs[inst.rs1()] * cpu.fp_regs[inst.rs2()];
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (_, 0xd) => {
      // fdiv.d
      cpu.fp_regs[inst.rd()] = cpu.fp_regs[inst.rs1()] / cpu.fp_regs[inst.rs2()];
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x0, 0x11) => {
      // fsgnj.d
      // Produce a result that takes all bits except the sign bit from rs1. The result’s sign bit is rs2’s sign bit.
      // Extract the sign bit from rs2 and the magnitude from rs1

      const SIGN_MASK: u64 = 0x8000_0000_0000_0000;
      const MAGNITUDE_MASK: u64 = 0x7FFF_FFFF_FFFF_FFFF;

      // Extract the sign bit from rs2
      let sign_rs2 = cpu.fp_regs[inst.rs2()].to_bits() & SIGN_MASK;

      // Extract the magnitude (all bits except the sign bit) from rs1
      let magnitude_rs1 = cpu.fp_regs[inst.rs1()].to_bits() & MAGNITUDE_MASK;

      // Combine the sign bit of rs2 with the magnitude of rs1
      trace!("fsgnj.d {}", f64::from_bits(sign_rs2 | magnitude_rs1));
      cpu.fp_regs[inst.rd()] = f64::from_bits(sign_rs2 | magnitude_rs1);
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x2, 0x11) => {
      // fsgnjx.d
      let rs1_bits = cpu.fp_regs[inst.rs1()].to_bits();
      let rs2_bits = cpu.fp_regs[inst.rs2()].to_bits();

      // Extract the sign bit from both rs1 and rs2
      let sign_bit_rs1 = rs1_bits & 0x8000_0000_0000_0000;
      let sign_bit_rs2 = rs2_bits & 0x8000_0000_0000_0000;

      // Compute the XOR of the sign bits
      let new_sign_bit = (sign_bit_rs1 ^ sign_bit_rs2) & 0x8000_0000_0000_0000;

      // Combine the new sign bit with the remaining bits of rs1
      let result_bits = (rs1_bits & 0x7FFF_FFFF_FFFF_FFFF) | new_sign_bit;

      trace!(
        "fsgnjx.d {}, {} -> {}",
        cpu.fp_regs[inst.rs1()],
        cpu.fp_regs[inst.rs2()],
        f64::from_bits(result_bits)
      );
      cpu.fp_regs[inst.rd()] = f64::from_bits(result_bits);
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x1, 0x11) => {
      // fsgnjn.d
      const SIGN_MASK: u64 = 0x8000_0000_0000_0000;
      const MAGNITUDE_MASK: u64 = 0x7FFF_FFFF_FFFF_FFFF;

      // Extract the sign bit from rs2
      let sign_rs2 = !cpu.fp_regs[inst.rs2()].to_bits() & SIGN_MASK;

      // Extract the magnitude (all bits except the sign bit) from rs1
      let magnitude_rs1 = cpu.fp_regs[inst.rs1()].to_bits() & MAGNITUDE_MASK;

      // Combine the sign bit of rs2 with the magnitude of rs1
      trace!("fsgnjn.d {}", f64::from_bits(sign_rs2 | magnitude_rs1));
      cpu.fp_regs[inst.rd()] = f64::from_bits(sign_rs2 | magnitude_rs1);
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x0, 0x51) => {
      // fle.d
      // Performs a quiet less or equal comparison between floating-point registers rs1 and rs2 and record the Boolean result in integer register rd.
      cpu.regs[inst.rd()] = if cpu.fp_regs[inst.rs1()] <= cpu.fp_regs[inst.rs2()] { 1 } else { 0 };
      trace!("{} <= {}: {}", cpu.fp_regs[inst.rs1()], cpu.fp_regs[inst.rs2()], cpu.regs[inst.rd()]);
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x1, 0x51) => {
      // flt.d
      // Performs a quiet less comparison between floating-point registers rs1 and rs2 and record the Boolean result in integer register rd.
      cpu.regs[inst.rd()] = if cpu.fp_regs[inst.rs1()] < cpu.fp_regs[inst.rs2()] { 1 } else { 0 };
      trace!("{} < {}: {}", cpu.fp_regs[inst.rs1()], cpu.fp_regs[inst.rs2()], cpu.regs[inst.rd()]);
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x2, 0x51) => {
      // feq.d
      // Performs a quiet equal comparison between floating-point registers rs1 and rs2 and record the Boolean result in integer register rd.
      // Only signaling NaN inputs cause an Invalid Operation exception.
      // The result is 0 if either operand is NaN.
      cpu.regs[inst.rd()] = if cpu.fp_regs[inst.rs1()] == cpu.fp_regs[inst.rs2()] { 1 } else { 0 };
      trace!("{} == {}: {}", cpu.fp_regs[inst.rs1()], cpu.fp_regs[inst.rs2()], cpu.regs[inst.rd()]);
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x0, 0x71) => {
      // fmv.x.d
      cpu.regs[inst.rd()] = cpu.fp_regs[inst.rs1()].to_bits();
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x0, 0x79) => {
      // fmv.d.x
      cpu.fp_regs[inst.rd()] = f64::from_bits(cpu.regs[inst.rs1()]);
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x0, 0x15) => {
      // fmin.d
      cpu.fp_regs[inst.rd()] = cpu.fp_regs[inst.rs1()].min(cpu.fp_regs[inst.rs2()]);
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x1, 0x15) => {
      // fmax.d
      cpu.fp_regs[inst.rd()] = cpu.fp_regs[inst.rs1()].max(cpu.fp_regs[inst.rs2()]);
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (_, 0x20) if inst.rs2() == 0x1 => {
      // fcvt.s.d
      // Should this be no-op?
      cpu.fp_regs[inst.rd()] = cpu.fp_regs[inst.rs1()];
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (_, 0x21) if inst.rs2() == 0x0 => {
      // fcvt.d.s
      cpu.fp_regs[inst.rd()] = cpu.fp_regs[inst.rs1()] as f32 as f64;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (_, 0x2c) => {
      // fsqrt.s
      // Perform single-precision square root.
      cpu.fp_regs[inst.rd()] = (cpu.fp_regs[inst.rs1()] as f32).sqrt() as f64;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    },
    (0x7, 0x69) => {
      // fcvt.d.l
      cpu.fp_regs[inst.rd()] = cpu.regs[inst.rs1()] as i64 as f64;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (_, 0x61) if inst.rs2() == 0x0 => {
      // fcvt.w.d
      cpu.regs[inst.rd()] = cpu.fp_regs[inst.rs1()] as i32 as u64;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (_, 0x61) if inst.rs2() == 0x2 => {
      // fcvt.l.d
      cpu.regs[inst.rd()] = cpu.fp_regs[inst.rs1()] as i64 as u64;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (_, 0x61) if inst.rs2() == 0x3 => {
      // fcvt.lu.d
      cpu.regs[inst.rd()] = cpu.fp_regs[inst.rs1()] as u64;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (_, 0x69) if inst.rs2() == 0x0 => {
      // fcvt.d.w
      cpu.fp_regs[inst.rd()] = cpu.regs[inst.rs1()] as i32 as f64;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (_, 0x69) if inst.rs2() == 0x1 => {
      // fcvt.d.wu
      cpu.fp_regs[inst.rd()] = cpu.regs[inst.rs1()] as u32 as f64;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    (0x0, 0x70) => {
      // fpow, non-standard
      cpu.fp_regs[inst.rd()] = cpu.fp_regs[inst.rs1()].powf(cpu.fp_regs[inst.rs2()]);
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    },
    (0x1, 0x70) => {
      // fcbrt, non-standard
      cpu.fp_regs[inst.rd()] = cpu.fp_regs[inst.rs1()].cbrt();
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    },
    (0x0, 0x72) => {
      // fsin, non-standard
      cpu.fp_regs[inst.rd()] = cpu.fp_regs[inst.rs1()].sin();
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    },
    (0x1, 0x72) => {
      // fcos, non-standard
      cpu.fp_regs[inst.rd()] = cpu.fp_regs[inst.rs1()].cos();
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    },
    (0x2, 0x72) => {
      // fatan2, non-standard
      cpu.fp_regs[inst.rd()] = cpu.fp_regs[inst.rs1()].atan2(cpu.fp_regs[inst.rs2()]);
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    },
    (0x0, 0x73) => {
      // frem, non-standard
      cpu.fp_regs[inst.rd()] = cpu.fp_regs[inst.rs1()].rem(cpu.fp_regs[inst.rs2()]);
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    },
    (0x1, 0x73) => {
      // fround, non-standard
      cpu.fp_regs[inst.rd()] = cpu.fp_regs[inst.rs1()].round();
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    },
    _ => {
      cpu.perf.end_cpu_time();
      Err(Exception::IllegalInstruction(*inst))
    }
  }
}
