use std::sync::Arc;

use async_trait::async_trait;
use mizu_hal_types::syscall;
use runtime::cpu::{Cpu, InterruptHandler};
use tokio::sync::oneshot;
use tracing::{debug, info};

use crate::environment::discord_ex::DiscordExInterruptHandler;
use crate::environment::dump_performance::DumpPerformanceHandler;
use crate::environment::halt::HaltHandler;
use crate::environment::http::HttpHandler;
use crate::environment::interrupt::IntHandler;
use crate::environment::log::LogHandler;
use crate::environment::png::PngHandler;
use crate::environment::time::TimeHandler;
use crate::execution_context::ExecutionContext;

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
      cpu.ivt.insert(
        syscall::SYSCALL_DISCORD_EX,
        Arc::new(Box::new(DiscordExInterruptHandler { context: self.context.clone() })),
      );
      cpu.ivt.insert(
        syscall::SYSCALL_PERF_DUMP,
        Arc::new(Box::new(DumpPerformanceHandler { context: self.context.clone() })),
      );
      cpu
        .ivt
        .insert(syscall::SYSCALL_HTTP, Arc::new(Box::new(HttpHandler { context: self.context.clone() })));
      cpu
        .ivt
        .insert(syscall::SYSCALL_LOG, Arc::new(Box::new(LogHandler { context: self.context.clone() })));
      cpu.ivt.insert(syscall::SYSCALL_HALT, Arc::new(Box::new(HaltHandler {})));
      cpu.ivt.insert(syscall::SYSCALL_TIME, Arc::new(Box::new(TimeHandler {})));
      cpu
        .ivt
        .insert(syscall::SYSCALL_SIPI, Arc::new(Box::new(SipiHandler { context: self.context.clone() })));
      cpu
        .ivt
        .insert(syscall::SYSCALL_INT, Arc::new(Box::new(IntHandler { context: self.context.clone() })));
      cpu.ivt.insert(syscall::SYSCALL_PNG, Arc::new(Box::new(PngHandler {})));
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
