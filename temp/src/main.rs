#![feature(lang_items)]
#![feature(naked_functions)]

#![no_std]
#![no_main]

mod prelude;

use prelude::*;

use mizu_hal::discord::*;
use mizu_hal::discord::discord::*;
use mizu_hal::discord::discord::discord_ex_request::DiscordExRequestUnion;
use mizu_hal::discord::prost::Message;
use mizu_hal::discord::prost::alloc::string::ToString;

#[link_section = ".start"]
#[no_mangle]
pub unsafe extern "C" fn _start() {
  __init_ivt_vector(core::ptr::addr_of!(__IVT_START));
  loop {
    __wfi();
  }
  halt();
}

pub unsafe extern "C" fn int_discord() {
  let len: u64;
  let ptr: *const core::ffi::c_void;
  asm!("", out("a0") len, out("a1") ptr);
  println!("discord interrupt: len={}, ptr={:#x}", len, ptr as u64);
  let data = core::slice::from_raw_parts(ptr as *const u8, len as usize);
  // println!("{:?}", data);
  let event = DiscordExEvent::decode(data).unwrap();
  println!("{:?}", event);
  asm!("mret");
}

pub unsafe extern "C" fn int_noop() {
  asm!("mret");
}

ivt! {
  1 => int_noop,
  17 => int_discord
}
