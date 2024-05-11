#![feature(lang_items)]
#![no_std]
#![no_main]

mod panic;
mod prelude;

use prelude::*;

#[link_section = ".start"]
#[no_mangle]
pub unsafe extern "C" fn _start() {
  println!("cpuid: ==={}===", read_null_terminated_string_unchecked(CPUID_NAME));
}
