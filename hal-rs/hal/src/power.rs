use core::arch::asm;

pub const POWERSTATE_DEFAULT: u64 = 1;
pub const POWERSTATE_BYPASS: u64 = 2;
pub const POWERSTATE_LOW_PRIORITY: u64 = 3;

pub unsafe fn __set_power_state(state: u64) {
  asm!("csrrw zero, 0x7C0, t0", in("t0") state);
}
