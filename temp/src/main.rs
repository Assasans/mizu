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
  println!("control returned to _start");
  halt();
}

pub unsafe extern "C" fn ivth() {
  // asm!("nop");
  println!("got interrupt from runtime");
  performance_dump();
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
