#![feature(lang_items)]
#![feature(naked_functions)]

#![no_std]
#![no_main]

mod panic;
mod prelude;

use prelude::*;

#[link_section = ".start"]
#[no_mangle]
pub unsafe extern "C" fn _start() {
  asm!(
    "li t0, 0xffffffff80000201",
    "csrrw zero, mtvec, t0"
  );
  halt();
}

pub unsafe extern "C" fn int_discord() {
  let ptr: *const core::ffi::c_char;
  asm!("", out("a0") ptr);
  println!("discord interrupt: ptr={}, content={:?}", ptr as u64, read_null_terminated_string_unchecked(ptr));
  asm!("mret");
}

global_asm!(r"
.section .text.ivt
.org .text.ivt + 17*4
  jal {}
", sym int_discord);
