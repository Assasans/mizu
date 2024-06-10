#![no_std]

use core::ffi::c_char;

pub mod discord;

#[repr(transparent)]
#[derive(Debug)]
pub struct StringPtr(pub *const c_char);

impl StringPtr {
  pub fn is_null(&self) -> bool {
    self.0.is_null()
  }
}

// SAFETY: xd :)
unsafe impl Send for StringPtr {}
