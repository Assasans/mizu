use core::arch::asm;
use core::ffi::c_void;

extern "C" {
  pub static __IVT_START: c_void;
}

pub unsafe fn __init_ivt_vector(address: *const c_void) {
  let mut pointer = address as u64;
  pointer |= 1; // Use vector mode

  asm!(
  "csrrw zero, mtvec, t0",
  in("t0") pointer
  );
}

pub unsafe fn __wfi() {
  asm!("wfi");
}
