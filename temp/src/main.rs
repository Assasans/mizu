#![feature(lang_items)]
#![feature(naked_functions)]

#![no_std]
#![no_main]

mod prelude;

use prelude::*;

#[link_section = ".start"]
#[no_mangle]
pub unsafe extern "C" fn _start() {
  asm!("csrrw zero, 0x320, t0", in("t0") 2);
  let addr = _ap_startup as *const ();
  println!("ap startup addr: {:?}", addr);
  asm!("", in("a0") addr);
  syscall(17);
  halt();
}

pub unsafe extern "C" fn _ap_startup() {
  asm!("csrrw zero, 0x320, t0", in("t0") 2);
  println!("Hello, 水の世界！");
  let fiba = fibonacci(25);
  println!("{fiba}");
  halt();
}

fn fibonacci(n: u32) -> u32 {
  match n {
    0 => 1,
    1 => 1,
    _ => fibonacci(n - 1) + fibonacci(n - 2),
  }
}
