use tracing::{debug, info};

use crate::cpu::{Cpu, Instruction};
use crate::csr;
use crate::exception::Exception;

#[inline(always)]
pub async fn system(inst: Instruction, cpu: &mut Cpu) -> Result<u64, Exception> {
  let csr_addr = ((*inst & 0xfff00000) >> 20) as usize;
  match inst.funct3() {
    0x0 => {
      match (inst.rs2(), inst.funct7()) {
        // ECALL and EBREAK cause the receiving privilege mode’s epc register to be set to the address of
        // the ECALL or EBREAK instruction itcpu, not the address of the following instruction.
        (0x0, 0x0) => {
          // ecall
          let num = cpu.regs[17];
          debug!("executing ecall {}", num);
          if let Some(handler) = cpu.ivt.get(&num) {
            cpu.perf.end_cpu_time();
            let handler = handler.clone();
            // syscalls are not cpu time limited
            handler.handle(cpu).await;
            cpu.perf.start_cpu_time();
          } else {
            return Err(Exception::RuntimeFault(num));
          }
          cpu.perf.end_cpu_time();
          cpu.update_pc()
        }
        (0x1, 0x0) => {
          // ebreak
          // Makes a request of the debugger bu raising a Breakpoint exception.
          cpu.perf.end_cpu_time();
          Err(Exception::Breakpoint(cpu.pc))
        }
        (0x2, 0x18) => {
          // mret
          if cpu.csr.load(csr::MCAUSE) == 0 {
            cpu.perf.end_cpu_time();
            return Err(Exception::RuntimeFault(333));
          }

          // Restore registers
          cpu.regs.swap_with_slice(&mut cpu.saved_regs);
          // cpu.regs.copy_from_slice(&cpu.saved_regs);
          // cpu.saved_regs.fill(0);

          debug!("trap exit: 0x{:x} -> 0x{:x}", cpu.pc, cpu.csr.load(csr::MEPC));
          cpu.pc = cpu.csr.load(csr::MEPC);
          cpu.csr.store(csr::MEPC, 0);
          cpu.csr.store(csr::MCAUSE, 0);
          cpu.csr.store(csr::MTVAL, 0);

          // TODO(Assasans): I have no idea what to do with MSTATUS
          let mut status = cpu.csr.load(csr::MSTATUS);

          let ie = (status & csr::MASK_MPIE) >> 7;
          // set MIE = MPIE
          status = (status & !csr::MASK_MIE) | (ie << 3);
          // set MPIE = 0
          status &= !csr::MASK_MPIE;
          cpu.csr.store(csr::MSTATUS, status);

          cpu.perf.end_cpu_time();
          Ok(cpu.pc)
          // return cpu.update_pc();
        }
        (0x5, 0x8) => {
          // wfi
          info!("waiting for interrupt");
          cpu.wfi.set(true);
          cpu.perf.end_cpu_time();
          cpu.update_pc()
        }
        (_, 0x9) => {
          // sfence.vma
          // Do nothing.
          cpu.perf.end_cpu_time();
          cpu.update_pc()
        }
        _ => {
          cpu.perf.end_cpu_time();
          Err(Exception::IllegalInstruction(*inst))
        }
      }
    }
    0x1 => {
      // csrrw
      let t = cpu.csr.load(csr_addr);
      cpu.csr.store(csr_addr, cpu.regs[inst.rs1()]);
      cpu.regs[inst.rd()] = t;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    0x2 => {
      // csrrs
      let t = cpu.csr.load(csr_addr);
      cpu.csr.store(csr_addr, t | cpu.regs[inst.rs1()]);
      cpu.regs[inst.rd()] = t;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    0x3 => {
      // csrrc
      let t = cpu.csr.load(csr_addr);
      cpu.csr.store(csr_addr, t & (!cpu.regs[inst.rs1()]));
      cpu.regs[inst.rd()] = t;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    0x5 => {
      // csrrwi
      let zimm = inst.rs1() as u64;
      cpu.regs[inst.rd()] = cpu.csr.load(csr_addr);
      cpu.csr.store(csr_addr, zimm);
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    0x6 => {
      // csrrsi
      let zimm = inst.rs1() as u64;
      let t = cpu.csr.load(csr_addr);
      cpu.csr.store(csr_addr, t | zimm);
      cpu.regs[inst.rd()] = t;
      cpu.perf.end_cpu_time();
      cpu.update_pc()
    }
    0x7 => {
      // csrrci
      let zimm = inst.rs1() as u64;
      let t = cpu.csr.load(csr_addr);
      cpu.csr.store(csr_addr, t & (!zimm));
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
