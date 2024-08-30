use std::collections::BTreeMap;
use std::ops::RangeInclusive;

use crate::bus::Bus;
use crate::exception::Exception;

pub type LoadFn = fn(&Bus, u64, RangeInclusive<u64>, u64) -> Result<u64, Exception>;
pub type StoreFn = fn(&Bus, u64, RangeInclusive<u64>, u64, u64) -> Result<(), Exception>;

pub struct AddressDecoderEntry {
  pub load: LoadFn,
  pub store: StoreFn,
}

pub struct AddressDecoder {
  segments: BTreeMap<u64, (u64, AddressDecoderEntry)>,
}

impl AddressDecoder {
  pub fn new() -> Self {
    Self { segments: BTreeMap::new() }
  }

  pub fn insert(&mut self, range: RangeInclusive<u64>, entry: AddressDecoderEntry) {
    let (start, end) = range.into_inner();
    self.segments.insert(start, (end, entry));
  }

  pub fn lookup(&self, address: u64) -> Option<(RangeInclusive<u64>, &AddressDecoderEntry)> {
    if let Some((start, (end, entry))) = self.segments.range(..=address).next_back() {
      if address <= *end {
        return Some((*start..=*end, entry));
      }
    }
    None
  }
}
