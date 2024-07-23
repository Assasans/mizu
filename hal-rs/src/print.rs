use core::fmt::{self, Write};

use crate::debug::Writer;

#[macro_export]
macro_rules! println {
  () => ($crate::print!("\n"));
  ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! print {
  ($($arg:tt)*) => ($crate::print::_print(format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
  let mut writer = Writer::new();
  writer.write_fmt(args).unwrap();
  writer.flush();
}
