use core::{arch::asm, ffi::c_void, ptr};

use crate::{syscall, SYSCALL_DISCORD};

pub use hal_types::discord::*;

pub unsafe fn discord_syscall(action: u64, data: *const c_void) -> *const c_void {
  asm!(
    "",
    in("a0") action,
    in("a1") data
  );
  syscall(SYSCALL_DISCORD);

  let result;
  asm!("", out("a0") result);

  result
}

pub fn create_message(message: &discord_create_message_t) -> discord_message_t {
  unsafe {
    let result = discord_syscall(action::CREATE_MESSAGE, message as *const discord_create_message_t as *const core::ffi::c_void);
    ptr::read(result as *const _)
  }
}

pub fn get_user(user_id: u64) -> discord_user_t {
  let request = discord_get_user_t {
    user_id
  };
  unsafe {
    let result = discord_syscall(action::GET_USER, &request as *const discord_get_user_t as *const core::ffi::c_void);
    ptr::read(result as *const _)
  }
}
