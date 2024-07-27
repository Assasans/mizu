use std::time::Duration;

use minstant::Instant;

pub const CPU_TIME_LIMIT: Duration = Duration::from_millis(10);

#[derive(Debug)]
pub struct PerformanceCounter {
  pub cpu_time: Duration,
  cpu_time_start: Option<Instant>,
  pub instructions_retired: u64,
}

impl Default for PerformanceCounter {
  fn default() -> Self {
    Self::new()
  }
}

impl PerformanceCounter {
  pub fn new() -> Self {
    PerformanceCounter {
      cpu_time: Duration::default(),
      cpu_time_start: None,
      instructions_retired: 0,
    }
  }

  pub fn reset(&mut self) {
    self.cpu_time = Duration::default();
    self.cpu_time_start = None;
    self.instructions_retired = 0;
  }

  pub fn start_cpu_time(&mut self) {
    assert!(self.cpu_time_start.is_none());
    self.cpu_time_start = Some(Instant::now());
  }

  pub fn end_cpu_time(&mut self) {
    let now = Instant::now();
    let start = self.cpu_time_start.take().unwrap();
    // let elapsed = ((now.0 - start.0) as f64 * minstant::nanos_per_cycle()) as u64;
    self.cpu_time += now - start;
  }
}
