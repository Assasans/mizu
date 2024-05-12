use core::ffi::c_void;

const START: *mut c_void = 0xffffffff80008000 as *mut c_void;

static mut POSITION: *mut c_void = START;

pub fn malloc(size: usize) -> *mut c_void {
  unsafe {
    let ptr = POSITION;
    POSITION = POSITION.add(size);
    ptr
  }
}
