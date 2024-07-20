use core::fmt::Write;
use core::panic::PanicInfo;
use crate::debug::Writer;
use crate::halt;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
  let mut writer = Writer::new();
  writer.write_fmt(format_args!("{}", info)).unwrap();

  #[cfg(feature = "backtrace")]
  {
    use mini_backtrace::Backtrace;

    writer.write_fmt(format_args!("\nstack backtrace:\n", )).unwrap();
    let backtrace = Backtrace::<16>::capture();
    for (index, frame) in backtrace.frames.iter().enumerate() {
      writer.write_fmt(format_args!("  {}: {:#x}\n", index, frame)).unwrap();
    }
    if backtrace.frames_omitted {
      writer.write_str("  ... <frames omitted>").unwrap();
    }
  }

  writer.flush();

  halt();
}
