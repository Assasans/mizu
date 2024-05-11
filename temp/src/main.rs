#![feature(lang_items)]
#![no_std]
#![no_main]

mod panic;
mod prelude;

use prelude::*;

#[link_section = ".start"]
#[no_mangle]
pub extern "C" fn _start() {
  panic!("xd");
}
