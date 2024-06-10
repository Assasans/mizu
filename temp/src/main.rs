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
  let mut count = 0;
  for i in 0..10000 {
    if i % 7 == 0 { count += 1; }
  }
  println!("{}", count);
  halt();
}
