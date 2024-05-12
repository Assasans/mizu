#![feature(lang_items)]
#![feature(naked_functions)]

#![no_std]
#![no_main]

mod panic;
mod prelude;

use prelude::*;

fn react(channel_id: u64, message_id: u64, emoji: &str) {
  unsafe {
    let result = discord::discord_syscall(
      discord::action::CREATE_REACTION,
      &discord::discord_create_reaction_t {
        channel_id,
        message_id,
        emoji: emoji.as_ptr() as *const core::ffi::c_char
      } as *const discord::discord_create_reaction_t as *const core::ffi::c_void
    );

    core::ptr::read(result as *const _);
  }
}

#[link_section = ".start"]
#[no_mangle]
pub unsafe extern "C" fn _start() {
  react(1173644182062116956, 1239250788870787152, "🤣");

  halt();
}
