use core::arch::asm;
use core::ffi::c_void;

extern "C" {
  pub static __ivt_start: c_void;
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
      ::core::arch::global_asm!(
        ".section .text.ivt",
        concat!(".org .text.ivt + ", $index, " * 4"),
        "jal {}",
        sym $handler
      );
    )*
  };
}

// This function must always be inlined otherwise the [a0] register will be overwritten.
#[inline(always)]
pub unsafe fn __save_registers() -> [u64; 8] {
  let mut a0: u64;
  let mut a1: u64;
  let mut a2: u64;
  let mut a3: u64;
  let mut a4: u64;
  let mut a5: u64;
  let mut a6: u64;
  let mut a7: u64;

  asm!(
  "",
  out("a0") a0,
  out("a1") a1,
  out("a2") a2,
  out("a3") a3,
  out("a4") a4,
  out("a5") a5,
  out("a6") a6,
  out("a7") a7,
  );

  [a0, a1, a2, a3, a4, a5, a6, a7]
}
