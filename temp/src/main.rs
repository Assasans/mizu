#![feature(lang_items)]
#![feature(naked_functions)]

#![no_std]
#![no_main]

mod prelude;

use prelude::*;

#[link_section = ".start"]
#[no_mangle]
pub unsafe extern "C" fn _start() {
  let val = core::sync::atomic::AtomicU64::new(0x55);
  val.fetch_or(0xaa, core::sync::atomic::Ordering::Relaxed);
  println!("{:#02x}", val.load(core::sync::atomic::Ordering::Relaxed));
  halt();
}
