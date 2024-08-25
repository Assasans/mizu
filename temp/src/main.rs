#![feature(lang_items)]
#![feature(naked_functions)]
#![allow(unused, internal_features)]

#![no_std]
#![no_main]

#[macro_use]
extern crate alloc;

mod prelude;

use prelude::*;

#[no_mangle]
pub unsafe fn main() {
  __set_power_state(POWERSTATE_BYPASS);
  let a = "124321";
  println!("{}", a);
  ab();
}

#[inline(never)]
unsafe fn ab() {
  __expl();
}
