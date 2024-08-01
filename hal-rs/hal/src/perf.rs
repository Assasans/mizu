use core::arch::asm;

pub unsafe fn __minstret() -> u64 {
  let instructions_retired: u64;
  asm!(
  "csrrs {0}, minstret, x0",
  out(reg) instructions_retired
  );
  instructions_retired
}
