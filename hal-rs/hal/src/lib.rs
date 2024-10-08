#![no_std]
#![allow(warnings, unused)]

#![feature(naked_functions)]

pub mod debug;
pub mod print;
pub mod panic;
pub mod alloc;
pub mod rand;
pub mod discord;
pub mod ivt;
pub mod power;
pub mod time;
pub mod device;
pub mod perf;

pub use mizu_hal_types as types;
pub use mini_backtrace as mini_backtrace;

use core::{arch::asm, ffi::c_char, ptr};
use mizu_hal_types::StringPtr;
use mizu_hal_types::syscall::*;

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
pub extern "C" fn memset(s: *mut u8, c: i32, mut len: usize) -> *mut u8 {
  let val64 = (c as u8 as u64).wrapping_mul(0x0101010101010101);
  let mut dest64 = s as *mut u64;

  while len >= 8 {
    unsafe {
      ptr::write(dest64, val64);
      dest64 = dest64.offset(1);
    }
    len -= 8;
  }

  let mut dest8 = dest64 as *mut u8;
  while len > 0 {
    unsafe {
      ptr::write(dest8, c as u8);
      dest8 = dest8.offset(1);
    }
    len -= 1;
  }

  s
}

#[no_mangle]
pub extern "C" fn memcmp(b: *const u8, c: *const u8, len: usize) -> i32 {
  let mut p = b;
  let mut q = c;

  let mut len = len;

  while len > 0 {
    unsafe {
      if *p != *q {
        return (*p as i32) - (*q as i32);
      }
      len -= 1;
      p = p.offset(1);
      q = q.offset(1);
    }
  }

  0
}

#[no_mangle]
pub extern "C" fn memcpy(dst: *mut u8, src: *const u8, mut n: usize) -> *mut u8 {
  let mut d = dst;
  let mut s = src;

  while n >= 8 {
    unsafe {
      ptr::write(d as *mut u64, ptr::read(s as *const u64));
      d = d.offset(8);
      s = s.offset(8);
    }
    n -= 8;
  }

  while n > 0 {
    unsafe {
      ptr::write(d, ptr::read(s));
      d = d.offset(1);
      s = s.offset(1);
    }
    n -= 1;
  }

  dst
}

#[no_mangle]
pub extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
  if n == 0 || dest == src as *mut u8 {
    return dest;
  }

  if dest < src as *mut u8 {
    // Forward copy
    let mut i = 0;
    while i < n {
      unsafe {
        *dest.add(i) = *src.add(i);
      }
      i += 1;
    }
  } else {
    // Backward copy to handle overlap
    let mut i = n;
    while i != 0 {
      i -= 1;
      unsafe {
        *dest.add(i) = *src.add(i);
      }
    }
  }

  dest
}

#[no_mangle]
pub extern "C" fn fmod(a: f64, b: f64) -> f64 {
  unsafe {
    let mut res: f64;
    asm!(".insn r OP_FP, 0x0, 0x73, {out}, {a}, {b}", a = in(freg) a, b = in(freg) b, out = out(freg) res);
    res
  }
}

pub unsafe fn read_null_terminated_string_unchecked<'a>(ptr: *const c_char) -> &'a str {
  let mut len = 0;
  let mut current_ptr = ptr;
  while *current_ptr != 0 {
    len += 1;
    current_ptr = current_ptr.add(1);
  }

  let slice = core::slice::from_raw_parts(ptr as *const u8, len);
  core::str::from_utf8_unchecked(slice)
}

/// Explodes the execution context.
///
/// All cores will shut down and will not automatically restart at the next Discord event.
pub unsafe fn __expl() -> ! {
  // Emit 0x7ffffff
  asm!(".insn r 0x7f, 0x7, 0x3, x31, x31, x31");
  loop {}
}

pub trait PtrExt<T: ?Sized> {
  fn new(value: &T) -> Self;
  fn get(&self) -> &T;
}

impl PtrExt<str> for StringPtr {
  fn new(value: &str) -> Self {
    // TODO: This must create null terminated pointer
    StringPtr(value.as_ptr() as *const c_char)
  }

  fn get(&self) -> &str {
    unsafe { read_null_terminated_string_unchecked(self.0) }
  }
}

extern {
  fn main();
}

#[link_section = ".start"]
#[no_mangle]
#[naked]
pub unsafe extern "C" fn _start() {
  asm!(
  "li sp, 0xffffffff81020000", // Initialize the stack
  "j {}",
  sym _main,
  options(noreturn)
  );
}

pub unsafe fn _main() {
  main();
  halt();
}
