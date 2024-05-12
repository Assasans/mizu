#![feature(lang_items)]
#![feature(naked_functions)]

#![no_std]
#![no_main]

mod panic;
mod prelude;

use prelude::*;

use chrono::TimeZone;

#[link_section = ".start"]
#[no_mangle]
pub unsafe extern "C" fn _start() {
  let ptr = 0x13010 as *const u128;
  let nanos = *ptr; // core::time::Duration::from_nanos(*ptr);
  
  println!("system time: {:?}", chrono::Utc.timestamp_opt((nanos / 1000000000u128) as i64, (nanos % 1000000000u128) as u32));

  halt();
}
