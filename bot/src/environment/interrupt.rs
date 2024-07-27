use std::sync::Arc;

use async_trait::async_trait;
use runtime::apic::INTERRUPT_PRIORITY_NORMAL;
use runtime::cpu::{Cpu, InterruptHandler};
use runtime::interrupt::Interrupt;
use tracing::{debug, info};

use crate::execution_context::ExecutionContext;

pub struct IntHandler {
  pub context: Arc<ExecutionContext>,
}

#[async_trait]
impl InterruptHandler for IntHandler {
  async fn handle(&self, cpu: &mut Cpu) {
    let src_regs = &cpu.regs;
    let id = cpu.regs[10] as u16;
    debug!("send interrupt to core {}...", id);

    let isolate = cpu.isolate.as_ref().unwrap().upgrade().unwrap();
    let cpu = isolate.get_core(id);
    let mut cpu = cpu.lock().await;

    info!("dispatching machine software interrupt");
    cpu.regs[10] = src_regs[10];
    cpu.apic.dispatch(Interrupt::MachineSoftwareInterrupt, INTERRUPT_PRIORITY_NORMAL);
    info!("resetting wfi");
    cpu.wfi.set(false);
  }
}
