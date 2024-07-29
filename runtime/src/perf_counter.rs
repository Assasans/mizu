use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use minstant::Instant;

pub const CPU_TIME_LIMIT: Duration = Duration::from_millis(10);

#[derive(Debug)]
pub struct PerformanceCounter {
  pub cpu_time: Mutex<Duration>,
  cpu_time_start: Mutex<Option<Instant>>,
  pub instructions_retired: AtomicU64,
}

impl Default for PerformanceCounter {
  fn default() -> Self {
    Self::new()
  }
}

impl PerformanceCounter {
  #[must_use]
  pub fn new() -> Self {
    Self {
      cpu_time: Mutex::new(Duration::default()),
      cpu_time_start: Mutex::new(None),
      instructions_retired: AtomicU64::new(0),
    }
  }

  pub fn reset(&self) {
    *self.cpu_time.lock().unwrap() = Duration::default();
    *self.cpu_time_start.lock().unwrap() = None;
    self.instructions_retired.store(0, Ordering::Release);
  }

  pub fn start_cpu_time(&self) {
    let mut start = self.cpu_time_start.lock().unwrap();
    assert!(start.is_none());
    *start = Some(Instant::now());
  }

  pub fn end_cpu_time(&self) {
    let now = Instant::now();
    let start = self.cpu_time_start.lock().unwrap().take().unwrap();
    // let elapsed = ((now.0 - start.0) as f64 * minstant::nanos_per_cycle()) as u64;
    *self.cpu_time.lock().unwrap() += now - start;
  }
}
