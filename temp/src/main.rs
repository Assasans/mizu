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
  println!("random: `0x{:x}`", MemoryMappedRng::new().next_u64());
  halt();
}
