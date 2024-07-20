macro_rules! memory_segment {
  ($name:ident, $start:expr, $size:expr) => {
    paste::paste! {
      pub const [<$name _BASE>]: u64 = $start;
      pub const [<$name _SIZE>]: u64 = $size;
      pub const [<$name _END>]: u64 = [<$name _BASE>] + [<$name _SIZE>] - 1;
    }
  };
}

memory_segment!(DRAM, 0xffffffff_80000000, 1024 * 1024 * 128);
memory_segment!(HARDWARE, 0x10000, 0x20000);
memory_segment!(CPUID, 0x10000, 0x100);
memory_segment!(RANDOM, 0x12000, 0x100);
