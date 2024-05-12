use std::{ptr, slice};
use std::ffi::CString;
use std::mem::size_of;

use rand::{thread_rng, Rng, RngCore};
use tracing::{debug, error, trace};

use crate::dram::Dram;
use crate::exception::Exception;
use crate::param::{CPUID_BASE, CPUID_END, DRAM_BASE, DRAM_END, RANDOM_BASE, RANDOM_END};

pub struct Bus {
  pub dram: Dram,
}

impl Bus {
  pub fn new(code: Vec<u8>) -> Bus {
    Self { dram: Dram::new(code) }
  }

  pub fn load(&mut self, addr: u64, size: u64) -> Result<u64, Exception> {
    trace!("bus load at 0x{addr:x}");
    match addr {
      CPUID_BASE..=CPUID_END => {
        let offset = (addr - CPUID_BASE) as usize;
        return match offset {
          // name
          0x0..=0x99 => {
            let version = format!("mizu emulated risc-v runtime v{}", env!("CARGO_PKG_VERSION"));
            let bytes = version.as_bytes();
            if offset < bytes.len() {
              return Ok(bytes[offset] as u64);
            };
            Ok(0)
          }
          _ => Err(Exception::LoadAccessFault(addr))
        };
      }
      RANDOM_BASE..=RANDOM_END => {
        let mut random = [0u8; 8];
        thread_rng().fill_bytes(&mut random[..(size / 8) as usize]);
        Ok(u64::from_le_bytes(random))
      }
      DRAM_BASE..=DRAM_END => self.dram.load(addr, size),
      _ => {
        error!("invalid load at 0x{addr:x}");
        Err(Exception::LoadAccessFault(addr))
      }
    }
  }

  pub fn store(&mut self, addr: u64, size: u64, value: u64) -> Result<(), Exception> {
    debug!("writing {value:x} at {addr:x}");
    match addr {
      DRAM_BASE..=DRAM_END => self.dram.store(addr, size, value),
      _ => Err(Exception::StoreAMOAccessFault(addr)),
    }
  }
}

pub trait BusMemoryExt {
  fn read(&mut self, addr: u64, len: u64) -> Result<Vec<u8>, Exception>;
  fn read_struct<T>(&mut self, addr: u64) -> Result<T, Exception>;
  fn read_string(&mut self, addr: u64) -> Result<CString, Exception>;

  fn write(&mut self, addr: u64, value: &[u8]) -> Result<(), Exception>;
  fn write_struct<T>(&mut self, addr: u64, value: &T) -> Result<(), Exception>;
  fn write_string(&mut self, addr: u64, value: &str) -> Result<(), Exception>;
}

fn previous_power_of_two(value: u64) -> u64 {
  let value = value | (value >> 1);
  let value = value | (value >> 2);
  let value = value | (value >> 4);
  let value = value | (value >> 8);
  let value = value | (value >> 16);
  return value - (value >> 1);
}

impl BusMemoryExt for Bus {
  fn read(&mut self, addr: u64, len: u64) -> Result<Vec<u8>, Exception> {
    let mut result = Vec::with_capacity(len as usize);
    let mut remaining = len;
    let mut offset = 0;

    while remaining > 0 {
      let bytes_to_read = remaining.min(8);
      let bytes_to_read = previous_power_of_two(bytes_to_read);
      let bits_to_read = bytes_to_read * 8;

      let value = self.load(addr + offset, bits_to_read)?;

      for i in 0..bytes_to_read {
        let byte = ((value >> (i * 8)) & 0xFF) as u8;
        result.push(byte);
      }

      remaining -= bytes_to_read;
      offset += bytes_to_read;
    }

    Ok(result)
  }

  fn read_struct<T>(&mut self, addr: u64) -> Result<T, Exception> {
    let bytes = self.read(addr, size_of::<T>() as u64)?;
    assert_eq!(bytes.len(), size_of::<T>());
    Ok(unsafe { ptr::read(bytes.as_ptr() as *const _) })
  }

  fn read_string(&mut self, addr: u64) -> Result<CString, Exception> {
    let mut address = addr;
    let mut data = Vec::new();
    loop {
      let byte = self.load(address, 8).unwrap() as u8;
      data.push(byte);
      if byte == 0 {
        break;
      }
      address += 1;
    }

    Ok(CString::from_vec_with_nul(data).unwrap())
  }

  fn write(&mut self, addr: u64, value: &[u8]) -> Result<(), Exception> {
    let mut address = addr;
    for byte in value {
      self.store(address, 8, *byte as u64).unwrap();
      address += 1;
    }
    Ok(())
  }

  fn write_struct<T>(&mut self, addr: u64, value: &T) -> Result<(), Exception> {
    let bytes = unsafe {
      slice::from_raw_parts((value as *const T) as *const u8, size_of::<T>())
    };
    self.write(addr, bytes)
  }

  fn write_string(&mut self, addr: u64, value: &str) -> Result<(), Exception> {
    let mut address = addr;
    for byte in value.as_bytes() {
      self.store(address, 8, *byte as u64).unwrap();
      address += 1;
    }
    self.store(address, 8, 0).unwrap();
    Ok(())
  }
}
