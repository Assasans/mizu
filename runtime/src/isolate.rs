use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;
use crate::bus::Bus;
use crate::cpu::Cpu;

pub struct Isolate {
  pub bus: Arc<Bus>,
  pub cores: std::sync::Mutex<Vec<Arc<Mutex<Cpu>>>>
}

impl Isolate {
  pub fn new(bus: Arc<Bus>) -> Arc<Self> {
    let this = Arc::new(Self {
      bus,
      cores: std::sync::Mutex::new(Vec::new())
    });

    this.add_core(Cpu::new(this.bus.clone(), Some(Arc::downgrade(&this))));
    this
  }

  pub fn get_bootstrap_core(&self) -> Arc<Mutex<Cpu>> {
    self.cores.lock().unwrap()[0].clone()
  }

  pub fn add_core(&self, core: Cpu) {
    let mut cores =self.cores.lock().unwrap();
    cores.push(Arc::new(Mutex::new(core)));
    info!("added core {}", cores.len() - 1);
  }

  pub fn wake(&self) {
    info!("waking isolate");
  }
}
