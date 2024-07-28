use core::alloc::{GlobalAlloc, Layout};
use core::ffi::c_void;

const START: *mut c_void = 0xffffffff80032000 as *mut c_void;

static mut POSITION: *mut c_void = START;

pub fn malloc(size: usize) -> *mut c_void {
  unsafe {
    let ptr = POSITION;
    POSITION = POSITION.add(size);
    ptr
  }
}

pub struct MizuAllocator {
  position: *mut c_void,
}

unsafe impl Sync for MizuAllocator {}

unsafe impl GlobalAlloc for MizuAllocator {
  unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    let ptr = POSITION;
    POSITION = POSITION.add(layout.size());
    ptr as *mut u8
  }

  unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
  }
}

#[global_allocator]
static ALLOCATOR: MizuAllocator = MizuAllocator {
  position: START
};
