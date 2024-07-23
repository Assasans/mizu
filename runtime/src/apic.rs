use std::sync::Mutex;
use priority_queue::PriorityQueue;
use crate::interrupt::Interrupt;

pub const INTERRUPT_PRIORITY_NORMAL: u16 = 10;

/// Advanced Programmable Interrupt Controller.
pub struct Apic {
  queue: Mutex<PriorityQueue<Interrupt, u16>>
}

impl Apic {
  pub fn new() -> Self {
    Self {
      queue: Mutex::new(PriorityQueue::new())
    }
  }

  pub fn dispatch(&self, interrupt: Interrupt, priority: u16) {
    let mut queue = self.queue.lock().unwrap();
    queue.push(interrupt, priority);
  }

  pub fn get(&self) -> Option<Interrupt> {
    let mut queue = self.queue.lock().unwrap();
    match queue.pop() {
      Some((interrupt, _)) => Some(interrupt),
      None => None
    }
  }
}