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
  let id: u64;
  let ptr: *const discord::discord_event_add_reaction_t;
  asm!("", out("a0") id, out("a1") ptr);
  println!("discord interrupt: id={:x}, ptr={:x}, message={:?}, content={:?}", id, ptr as u64, *ptr, (*ptr).emoji.get());
  asm!("mret");
}

global_asm!(r"
.section .text.ivt
.org .text.ivt + 17*4
  jal {}
", sym int_discord);
