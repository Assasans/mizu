#![feature(lang_items)]
#![feature(naked_functions)]

#![no_std]
#![no_main]

mod prelude;

use prelude::*;

#[link_section = ".start"]
#[no_mangle]
pub unsafe extern "C" fn _start() {
  init_ivt_vector(core::ptr::addr_of!(__IVT_START));
  halt();
}

pub unsafe extern "C" fn int_discord() {
  let id: u64;
  let ptr: *const core::ffi::c_void;
  asm!("", out("a0") id, out("a1") ptr);
  println!("discord interrupt: id={:x}, ptr={:x}", id, ptr as u64);
  match id {
    discord::action::EVENT_MESSAGE_CREATE => {
      let ptr = ptr as *const discord::discord_message_t;
      println!("EVENT_MESSAGE_CREATE event={:?}", *ptr);
    }
    discord::action::EVENT_REACTION_ADD => {
      let ptr = ptr as *const discord::discord_event_add_reaction_t;
      println!("EVENT_REACTION_ADD event={:?}", *ptr);
    }
    _ => todo!("id={}", id)
  }
  asm!("mret");
}

global_asm!(r"
.section .text.ivt
.org .text.ivt + 17*4
  jal {}
", sym int_discord);
