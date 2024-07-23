use std::sync::Arc;
use async_trait::async_trait;
use runtime::bus::BusMemoryExt;
use runtime::interrupt::Interrupt;
use runtime::apic::INTERRUPT_PRIORITY_NORMAL;
use runtime::cpu::{Cpu, InterruptHandler};
use crate::execution_context::ExecutionContext;

pub struct DumpPerformanceHandler {
  pub context: Arc<ExecutionContext>,
}

#[async_trait]
impl InterruptHandler for DumpPerformanceHandler {
  async fn handle(&self, cpu: &mut Cpu) {
    let http = self.context.http.lock().await.as_ref().unwrap().clone();
    let channel_id = self.context.channel_id.lock().await.unwrap();

    http.create_message(channel_id)
      .content(&format!("performance dump: ```c\nperf={:?}\npc = 0x{:x}```", cpu.perf, cpu.pc)).unwrap()
      .await.unwrap();
    cpu.perf.reset();

    cpu.regs[10] = 0x50;
    cpu.apic.dispatch(Interrupt::PlatformDefined16, INTERRUPT_PRIORITY_NORMAL);
    let isolate = cpu.isolate.as_ref().unwrap().upgrade().unwrap();
    isolate.wake();

    // if cpu.csr.load(MCAUSE) == 0 {
    //   error!("exited from trap");
    //   break; // Exited from trap
    // }
    let ptr = cpu.saved_regs[10];
    cpu.saved_regs.fill(0);
    cpu.bus.write_string(ptr, "the fog is coming shit").unwrap();

    http.create_message(channel_id)
      .content(&format!("allocated: `0x{:x}`", ptr)).unwrap()
      .await.unwrap();
  }
}
