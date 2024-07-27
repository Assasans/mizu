#![feature(lang_items)]
#![feature(naked_functions)]

#![no_std]
#![no_main]

mod prelude;

use prelude::*;

#[link_section = ".start"]
#[no_mangle]
pub unsafe extern "C" fn _start() {
  __set_power_state(POWERSTATE_BYPASS);
  let addr = _ap_startup as *const ();
  println!("ap startup addr: {:?}", addr);
  asm!("", in("a0") addr);
  syscall(SYSCALL_SIPI);
  println!("ap started");
  
  let id = 1;
  asm!("", in("a0") id);
  syscall(SYSCALL_INT);

  halt();
}

pub unsafe extern "C" fn _ap_startup() {
  __set_power_state(POWERSTATE_BYPASS);
  println!("Hello, 水の世界！");
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
  println!("got msi: len={}, ptr={:#x}", len, ptr as u64);
  asm!("mret");
}

global_asm!(r"
.section .text.ivt
.org .text.ivt + 3*4
  jal {}
", sym int_discord);
