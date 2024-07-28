use core::arch::asm;
use mizu_hal_types::syscall::SYSCALL_PNG;
use crate::syscall;

pub fn encode(pixels: &[u8], width: u16, height: u16) -> &'static [u8] {
  let resolution = width as u32 | ((height as u32) << 16);
  unsafe {
    asm!(
    "",
    in("a0") pixels.len(),
    in("a1") pixels.as_ptr(),
    in("a2") resolution
    );

    syscall(SYSCALL_PNG);
    let mut len: u64;
    let mut ptr: *const u8;

    asm!(
    "",
    out("a0") len,
    out("a1") ptr
    );

    core::slice::from_raw_parts(ptr, len as usize)
  }
}
