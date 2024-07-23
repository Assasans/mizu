#![feature(lang_items)]
#![feature(naked_functions)]

#![no_std]
#![no_main]

mod prelude;

use prelude::*;

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
  let id: u64;
  let ptr: *const core::ffi::c_void;
  asm!("", out("a0") id, out("a1") ptr);
  println!("discord interrupt: id={:x}, ptr={:x}", id, ptr as u64);
  match id {
    discord::action::EVENT_MESSAGE_CREATE => {
      let ptr = ptr as *const discord::discord_message_t;
      let msg = &*ptr;
      let user = discord::get_user(msg.author_id);
      println!("{} ({}): {}", user.name.get(), user.global_name.get(), msg.content.get());
    }
    _ => {}
  }
  asm!("mret");
}

global_asm!(r"
.section .text.ivt
.org .text.ivt + 17*4
  jal {}
", sym int_discord);
