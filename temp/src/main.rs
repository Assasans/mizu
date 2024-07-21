#![feature(lang_items)]
#![feature(naked_functions)]

#![no_std]
#![no_main]

mod prelude;

use prelude::*;

#[link_section = ".start"]
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn _start() {
  asm!("csrrw zero, 0x320, t0", in("t0") 2);
  test_shit();
}

#[inline(never)]
fn test_shit() {
  inner();
}

#[inline(never)]
fn inner() {
  panic!("fuck");
}
