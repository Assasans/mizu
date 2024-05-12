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
  performance_dump();
  // read_null_terminated_string_unchecked(0xffffffff80008000 as *const core::ffi::c_char);
  println!("got {}", read_null_terminated_string_unchecked(0xffffffff80008000 as *const core::ffi::c_char));
  halt();
}

pub unsafe extern "C" fn ivth() {
  let size;
  asm!("", out("a0") size);
  let ptr = malloc(size);
  println!("allocated {} bytes: 0x{:x}", size, ptr as usize);
  asm!("", in("a0") ptr);
}

#[naked]
#[no_mangle]
#[link_section = ".text.ivt"]
pub unsafe extern "C" fn ivt() {
  asm!(
    ".org .text.ivt + 16*4",
    "jal {}",
    "mret",
    sym ivth,
    options(noreturn)
  );
}
