#![feature(lang_items)]
#![feature(naked_functions)]

#![no_std]
#![no_main]

mod prelude;

use prelude::*;

#[link_section = ".start"]
#[no_mangle]
pub unsafe extern "C" fn _start() {
  __init_ivt_vector(core::ptr::addr_of!(__IVT_START));
  __wfi();
  println!("absolute={:?} relative={:?}", now_absolute(), Instant::now());
  halt();
}

global_asm!(r"
.section .text.ivt
.org .text.ivt + 17*4
  mret
");
