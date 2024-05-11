#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
  loop {}
}

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}
