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
  syscall(17);
  halt();
}
