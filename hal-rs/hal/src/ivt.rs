use core::arch::{asm, global_asm};
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

#[macro_export]
macro_rules! ivt {
  ($($index:expr => $handler:ident),* $(,)?) => {
    $(
      global_asm!(
        ".section .text.ivt",
        concat!(".org .text.ivt + ", $index, " * 4"),
        "jal {}",
        sym $handler
      );
    )*
  };
}
