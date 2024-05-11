use core::fmt::Write;

use mizu_hal::debug::Writer;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
  let mut writer = Writer::new();
  writer.write_fmt(format_args!("{}", info)).unwrap();
  writer.flush();

  loop {}
}
