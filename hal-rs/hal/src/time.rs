use core::arch::asm;
use core::fmt;
use core::fmt::Debug;
use core::ops::Sub;
use core::time::Duration;
use mizu_hal_types::syscall::SYSCALL_TIME;
use crate::syscall;

pub fn now_relative() -> u64 {
  let mut time: u64;
  unsafe {
    asm!(
    "rdtime a0", out("a0") time
    );
  }
  time
}

pub fn now_absolute() -> u128 {
  let mut upper: u64;
  let mut lower: u64;
  unsafe {
    syscall(SYSCALL_TIME);
    asm!(
    "",
    out("a0") upper,
    out("a1") lower,
    );
  }

  ((upper as u128) << 64) | lower as u128
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant(u64);

impl Instant {
  #[must_use]
  pub fn now() -> Self {
    Self(now_relative())
  }

  #[must_use]
  pub fn duration_since(&self, earlier: Instant) -> Duration {
    self.checked_duration_since(earlier).unwrap_or_default()
  }

  #[must_use]
  pub fn checked_duration_since(&self, earlier: Instant) -> Option<Duration> {
    self.0.checked_sub(earlier.0).map(|diff| Duration::from_nanos(diff))
  }

  #[must_use]
  pub fn elapsed(&self) -> Duration {
    Instant::now() - *self
  }
}

impl Sub<Instant> for Instant {
  type Output = Duration;

  fn sub(self, other: Instant) -> Duration {
    self.duration_since(other)
  }
}

impl Debug for Instant {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let nanos = self.0;
    let secs = nanos / 1_000_000_000;
    let subsec_nanos = nanos % 1_000_000_000;
    let millis = subsec_nanos / 1_000_000;
    let sub_millis = subsec_nanos % 1_000_000;
    let micros = sub_millis / 1_000;
    let sub_micros = sub_millis % 1_000;

    if secs > 0 {
      write!(f, "{}.{:09}s", secs, subsec_nanos)
    } else if millis > 0 {
      write!(f, "{}.{:03}ms", millis, micros)
    } else if micros > 0 {
      write!(f, "{}.{:03}µs", micros, sub_micros)
    } else {
      write!(f, "{}ns", nanos)
    }
  }
}
