use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::oneshot;
use mizu_hal_types::syscall;
use runtime::cpu::{Cpu, InterruptHandler};
use tracing::{debug, info};
use crate::discord::DiscordInterruptHandler;
use crate::dump_performance::DumpPerformanceHandler;
use crate::execution_context::ExecutionContext;
use crate::halt::HaltHandler;
use crate::http::HttpHandler;
use crate::interrupt::IntHandler;
use crate::log::LogHandler;
use crate::object_storage::ObjectStorageHandler;
use crate::time::TimeHandler;

pub struct SipiHandler {
  pub context: Arc<ExecutionContext>,
}

#[async_trait]
impl InterruptHandler for SipiHandler {
  async fn handle(&self, cpu: &mut Cpu) {
    let pc = cpu.regs[10];
    debug!("creating new core, pc={:#18x}...", pc);

    let isolate = cpu.isolate.as_ref().unwrap().upgrade().unwrap();
    let id = isolate.cores.lock().unwrap().len() as u16;
    let cpu = isolate.add_core(Cpu::new(id, isolate.bus.clone(), Some(Arc::downgrade(&isolate))));

    {
      let mut cpu = cpu.lock().await;
      cpu.pc = pc;
      cpu.ivt.insert(syscall::SYSCALL_PERF_DUMP, Arc::new(Box::new(DumpPerformanceHandler { context: self.context.clone() })));
      cpu.ivt.insert(syscall::SYSCALL_HTTP, Arc::new(Box::new(HttpHandler { context: self.context.clone() })));
      cpu.ivt.insert(syscall::SYSCALL_LOG, Arc::new(Box::new(LogHandler { context: self.context.clone() })));
      cpu.ivt.insert(syscall::SYSCALL_HALT, Arc::new(Box::new(HaltHandler {})));
      cpu.ivt.insert(syscall::SYSCALL_TIME, Arc::new(Box::new(TimeHandler {})));
      cpu.ivt.insert(syscall::SYSCALL_SIPI, Arc::new(Box::new(SipiHandler { context: self.context.clone() })));
      cpu.ivt.insert(syscall::SYSCALL_INT, Arc::new(Box::new(IntHandler { context: self.context.clone() })));
    }

    let (cpu_ready_tx, cpu_ready_rx) = oneshot::channel::<()>();

    let context = self.context.clone();
    tokio::spawn(async move {
      context.run_core(cpu, Some(cpu_ready_tx)).await.unwrap();
    });

    cpu_ready_rx.await.unwrap();
    info!("core {} ready", id);
  }
}
