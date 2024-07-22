use std::sync::Arc;
use async_trait::async_trait;
use runtime::cpu::{Cpu, InterruptHandler};
use tracing::debug;

pub struct SipiHandler {
}

#[async_trait]
impl InterruptHandler for SipiHandler {
  async fn handle(&self, cpu: &mut Cpu) {
    debug!("creating new core...");
    let isolate = cpu.isolate.as_ref().unwrap().upgrade().unwrap();
    isolate.add_core(Cpu::new(isolate.bus.clone(), Some(Arc::downgrade(&isolate))));
  }
}
