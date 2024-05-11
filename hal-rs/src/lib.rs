#![no_std]

pub mod debug;

use core::arch::asm;

pub const SYSCALL_DISCORD: u64 = 10;
pub const SYSCALL_PERF_DUMP: u64 = 11;
pub const SYSCALL_HTTP: u64 = 12;
pub const SYSCALL_OBJECT_STORAGE: u64 = 13;
pub const SYSCALL_LOG: u64 = 14;
pub const SYSCALL_HALT: u64 = 15;

#[inline(always)]
pub unsafe fn syscall(number: u64) {
  asm!(
    "ecall",
    in("a7") number,
    options(nomem, nostack)
  );
}

pub fn performance_dump() {
  // SAFETY: Safe :)
  // No register changes needed
  unsafe { syscall(SYSCALL_PERF_DUMP); }
}

pub fn halt() -> ! {
  // SAFETY: Safe :)
  // No register changes needed
  unsafe { syscall(SYSCALL_HALT); }
  loop {}
}

pub fn debug_log(message: &str) {
  // SAFETY: Safe :)
  unsafe {
    asm!(
      "",
      in("a0") message.as_ptr(),
      options(nomem, nostack)
    );
    syscall(SYSCALL_LOG);
  }
}


pub fn debug_log_bytes(message: *const u8) {
  // SAFETY: Safe :)
  unsafe {
    asm!(
      "",
      in("a0") message,
      options(nomem, nostack)
    );
    syscall(SYSCALL_LOG);
  }
}

#[no_mangle]
pub extern "C" fn memset(s: *mut u8, c: i32, len: usize) -> *mut u8 {
  let mut dst = s;
  let mut remaining_len = len;

  while remaining_len > 0 {
    unsafe {
      *dst = c as u8;
    }
    dst = unsafe { dst.offset(1) };
    remaining_len -= 1;
  }

  s
}
