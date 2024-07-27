use std::collections::HashMap;
use std::ffi::c_char;
use std::ptr;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use runtime::bus::BusMemoryExt;
use runtime::cpu::{Cpu, InterruptHandler};
use runtime::memory::HARDWARE_BASE;
use tracing::debug;

use crate::execution_context::ExecutionContext;

pub struct ObjectStorage {
  data: RwLock<HashMap<String, Vec<u8>>>,
}

impl ObjectStorage {
  pub fn new() -> Self {
    Self {
      data: RwLock::new(HashMap::new()),
    }
  }

  pub fn get(&self, key: &str) -> Option<Vec<u8>> {
    self.data.read().unwrap().get(key).cloned()
  }

  pub fn put(&self, key: &str, value: &[u8]) {
    self.data.write().unwrap().insert(key.to_owned(), value.to_owned());
  }
}

pub struct ObjectStorageHandler {
  pub context: Arc<ExecutionContext>,
  pub object_storage: Arc<ObjectStorage>,
}

#[repr(C)]
#[derive(Debug)]
pub struct object_storage_get_t {
  pub key: *const c_char,
}

unsafe impl Send for object_storage_get_t {}

#[repr(C)]
#[derive(Debug)]
pub struct object_storage_put_t {
  pub key: *const c_char,
  pub item: object_storage_item_t,
}

unsafe impl Send for object_storage_put_t {}

#[repr(C)]
#[derive(Debug)]
pub struct object_storage_item_t {
  pub length: u64,
  pub data: *const c_char,
}

unsafe impl Send for object_storage_item_t {}

#[async_trait]
impl InterruptHandler for ObjectStorageHandler {
  async fn handle(&self, cpu: &mut Cpu) {
    let action = cpu.regs[10];
    let address = cpu.regs[11];
    match action {
      1 => {
        // get
        let request = cpu.bus.read_struct::<object_storage_get_t>(address).unwrap();
        debug!("request: {:?}", request);

        let key = cpu.bus.read_string(request.key as u64).unwrap().to_string_lossy().to_string();
        debug!("get by key: {}", key);

        let data = self.object_storage.get(&key).unwrap();

        let ffi_response = object_storage_item_t {
          length: data.len() as u64,
          data: (HARDWARE_BASE + 0x9900) as *const c_char,
        };

        cpu.bus.write(ffi_response.data as u64, &data).unwrap();
        cpu.bus.write_struct(HARDWARE_BASE + 0x6000, &ffi_response).unwrap();
        cpu.regs[10] = HARDWARE_BASE + 0x6000;
      }
      2 => {
        // put
        let request = cpu.bus.read_struct::<object_storage_put_t>(address).unwrap();
        debug!("request: {:?}", request);

        let key = cpu.bus.read_string(request.key as u64).unwrap().to_string_lossy().to_string();
        debug!("put by key: {}", key);

        debug!("data at {:x}", request.item.data as u64);
        let data = cpu.bus.read(request.item.data as u64, request.item.length).unwrap();
        self.object_storage.put(&key, &data);

        let ffi_response = object_storage_item_t {
          length: data.len() as u64,
          data: ptr::null(),
        };

        cpu.bus.write_struct(HARDWARE_BASE + 0x6000, &ffi_response).unwrap();
        cpu.regs[10] = HARDWARE_BASE + 0x6000;
      }
      _ => unimplemented!(),
    }
  }
}
