#![feature(lang_items)]
#![feature(naked_functions)]

#![no_std]
#![no_main]

mod panic;
mod prelude;

use prelude::*;

extern crate alloc;

use alloc::string::ToString;

#[link_section = ".start"]
#[no_mangle]
pub unsafe extern "C" fn _start() {
  println!("test: {:?}", "hi".to_string().to_uppercase());
  halt();
}
