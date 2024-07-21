use std::sync::Arc;
use tokio::sync::Mutex;
use crate::bus::Bus;
use crate::cpu::Cpu;

pub struct Isolate {
  pub bus: Arc<Bus>,
  pub cores: Vec<Mutex<Cpu>>
}

impl Isolate {
  pub fn new(bus: Arc<Bus>) -> Self {
    Self {
      bus: bus.clone(),
      cores: vec![Mutex::new(Cpu::new(bus.clone()))]
    }
  }

  pub fn get_bootstrap_core(&self) -> &Mutex<Cpu> {
    &self.cores[0]
  }
}
