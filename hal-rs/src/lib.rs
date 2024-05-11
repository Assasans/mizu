#![no_std]

use core::arch::asm;

pub const SYSCALL_DISCORD: u64 = 10;
pub const SYSCALL_PERF_DUMP: u64 = 11;
pub const SYSCALL_HTTP: u64 = 12;
pub const SYSCALL_OBJECT_STORAGE: u64 = 13;

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
