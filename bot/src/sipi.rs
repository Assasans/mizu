use std::sync::Arc;
use async_trait::async_trait;
use runtime::cpu::{Cpu, InterruptHandler};
use tracing::debug;
use crate::discord::DiscordInterruptHandler;
use crate::dump_performance::DumpPerformanceHandler;
use crate::execution_context::ExecutionContext;
use crate::halt::HaltHandler;
use crate::http::HttpHandler;
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
      cpu.ivt.insert(11, Arc::new(Box::new(DumpPerformanceHandler { context: self.context.clone() })));
      cpu.ivt.insert(12, Arc::new(Box::new(HttpHandler { context: self.context.clone() })));
      cpu.ivt.insert(14, Arc::new(Box::new(LogHandler { context: self.context.clone() })));
      cpu.ivt.insert(15, Arc::new(Box::new(HaltHandler {})));
      cpu.ivt.insert(16, Arc::new(Box::new(TimeHandler {})));
      cpu.ivt.insert(17, Arc::new(Box::new(SipiHandler { context: self.context.clone() })));
    }

    let context = self.context.clone();
    tokio::spawn(async move {
      context.run_core(cpu).await.unwrap();
    });
  }
}
