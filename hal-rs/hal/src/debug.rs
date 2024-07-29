use core::fmt;

use crate::debug_log_bytes;

pub struct Writer {
  position: usize,
  buffer: [u8; 1024],
}

impl Writer {
  pub fn new() -> Self {
    Writer {
      position: 0,
      buffer: [0; 1024]
    }
  }

  pub fn write_byte(&mut self, byte: u8) {
    if self.position >= self.buffer.len() - 2 {
      return;
    }

    self.buffer[self.position] = byte;
    self.position += 1;
  }

  pub fn write_string(&mut self, s: &str) {
    for byte in s.as_bytes() {
      self.write_byte(*byte);
    }
  }

  pub fn flush(&mut self) {
    self.position += 1;
    self.buffer[self.position] = 0;
    debug_log_bytes(self.buffer.as_ptr());
    self.position = 0;
  }
}

impl fmt::Write for Writer {
  fn write_str(&mut self, s: &str) -> fmt::Result {
    self.write_string(s);
    Ok(())
  }
}
