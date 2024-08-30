use std::ffi::CString;
use std::mem::size_of;
use std::ops::RangeInclusive;
use std::sync::RwLock;
use std::{ptr, slice};

use mizu_hwconst::memory::*;
use rand::{thread_rng, RngCore};
use tracing::{debug, error, trace};

use crate::address_decoder::{AddressDecoder, AddressDecoderEntry};
use crate::dram::Dram;
use crate::exception::Exception;

pub struct Bus {
  pub dram: RwLock<Dram>,
  pub hardware: RwLock<Vec<u8>>,
  pub address_decoder: RwLock<AddressDecoder>,
}

pub fn store_fail(_bus: &Bus, addr: u64, _range: RangeInclusive<u64>, _size: u64, _value: u64) -> Result<(), Exception> {
  Err(Exception::StoreAMOAccessFault(addr))
}

impl Bus {
  #[must_use]
  pub fn new(code: Vec<u8>) -> Self {
    let mut address_decoder = AddressDecoder::new();
    address_decoder.insert(DRAM_BASE..=DRAM_END, AddressDecoderEntry {
      load: |bus, addr, range, size| bus.dram.read().unwrap().load(addr - range.start(), size),
      store: |bus, addr, range, size, value| bus.dram.write().unwrap().store(addr - range.start(), size, value),
    });
    address_decoder.insert(HARDWARE_BASE..=HARDWARE_END, AddressDecoderEntry {
      load: |bus, addr, range, size| {
        if ![8, 16, 32, 64].contains(&size) {
          error!("unaligned load at 0x{addr:x}, {size}");
          return Err(Exception::LoadAccessFault(addr));
        }
        let nbytes = size / 8;
        let index = (addr - HARDWARE_BASE) as usize;
        let memory = bus.hardware.read().unwrap();
        let mut code = memory[index] as u64;
        // shift the bytes to build up the desired value
        for i in 1..nbytes {
          code |= (memory[index + i as usize] as u64) << (i * 8);
        }

        Ok(code)
      },
      store: |bus, addr, range, size, value| {
        if ![8, 16, 32, 64].contains(&size) {
          return Err(Exception::StoreAMOAccessFault(addr));
        }

        let nbytes = size / 8;
        let index = (addr - HARDWARE_BASE) as usize;
        for i in 0..nbytes {
          let offset = 8 * i as usize;
          let mut memory = bus.hardware.write().unwrap();
          memory[index + i as usize] = ((value >> offset) & 0xff) as u8;
        }
        Ok(())
      },
    });
    address_decoder.insert(RANDOM_BASE..=RANDOM_END, AddressDecoderEntry {
      load: |bus, addr, range, size| {
        let mut random = [0u8; 8];
        thread_rng().fill_bytes(&mut random[..(size / 8) as usize]);
        Ok(u64::from_le_bytes(random))
      },
      store: store_fail,
    });
    address_decoder.insert(CPUID_BASE..=CPUID_END, AddressDecoderEntry {
      load: |bus, addr, range, size| {
        let offset = (addr - range.start()) as usize;
        match offset {
          // name
          0x0..=0x99 => {
            let version = format!("mizu emulated risc-v runtime v{}", env!("CARGO_PKG_VERSION"));
            let bytes = version.as_bytes();
            if offset < bytes.len() {
              return Ok(bytes[offset] as u64);
            };
            Ok(0)
          }
          _ => Err(Exception::LoadAccessFault(addr)),
        }
      },
      store: store_fail,
    });

    Self {
      dram: RwLock::new(Dram::new(code)),
      hardware: RwLock::new(vec![0xaa; HARDWARE_SIZE as usize]),
      address_decoder: RwLock::new(address_decoder),
    }
  }

  pub fn load(&self, addr: u64, size: u64) -> Result<u64, Exception> {
    trace!("bus load at 0x{addr:x}");

    let address_decoder = self.address_decoder.read().unwrap();
    match address_decoder.lookup(addr) {
      Some((range, entry)) => (entry.load)(self, addr, range, size),
      None => {
        error!("invalid load at 0x{addr:x}");
        Err(Exception::LoadAccessFault(addr))
      }
    }
  }

  pub fn store(&self, addr: u64, size: u64, value: u64) -> Result<(), Exception> {
    debug!("writing {value:x} at {addr:x}");

    let address_decoder = self.address_decoder.read().unwrap();
    match address_decoder.lookup(addr) {
      Some((range, entry)) => (entry.store)(self, addr, range, size, value),
      None => Err(Exception::StoreAMOAccessFault(addr)),
    }
  }
}

pub trait BusMemoryExt {
  fn read(&self, addr: u64, len: u64) -> Result<Vec<u8>, Exception>;
  fn read_struct<T>(&self, addr: u64) -> Result<T, Exception>;
  fn read_string(&self, addr: u64) -> Result<CString, Exception>;

  fn write(&self, addr: u64, value: &[u8]) -> Result<(), Exception>;
  fn write_struct<T>(&self, addr: u64, value: &T) -> Result<(), Exception>;
  fn write_string(&self, addr: u64, value: &str) -> Result<(), Exception>;
}

const fn previous_power_of_two(value: u64) -> u64 {
  let value = value | (value >> 1);
  let value = value | (value >> 2);
  let value = value | (value >> 4);
  let value = value | (value >> 8);
  let value = value | (value >> 16);
  value - (value >> 1)
}

impl BusMemoryExt for Bus {
  fn read(&self, addr: u64, len: u64) -> Result<Vec<u8>, Exception> {
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

  fn read_struct<T>(&self, addr: u64) -> Result<T, Exception> {
    let bytes = self.read(addr, size_of::<T>() as u64)?;
    assert_eq!(bytes.len(), size_of::<T>());
    Ok(unsafe { ptr::read(bytes.as_ptr() as *const _) })
  }

  fn read_string(&self, addr: u64) -> Result<CString, Exception> {
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

  fn write(&self, addr: u64, value: &[u8]) -> Result<(), Exception> {
    let mut address = addr;
    for byte in value {
      self.store(address, 8, *byte as u64).unwrap();
      address += 1;
    }
    Ok(())
  }

  fn write_struct<T>(&self, addr: u64, value: &T) -> Result<(), Exception> {
    let bytes = unsafe { slice::from_raw_parts((value as *const T) as *const u8, size_of::<T>()) };
    self.write(addr, bytes)
  }

  fn write_string(&self, addr: u64, value: &str) -> Result<(), Exception> {
    let mut address = addr;
    for byte in value.as_bytes() {
      self.store(address, 8, *byte as u64).unwrap();
      address += 1;
    }
    self.store(address, 8, 0).unwrap();
    Ok(())
  }
}
