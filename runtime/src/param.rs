pub const DRAM_BASE: u64 = 0xffffffff_80000000; // 0x8000_0000;
pub const DRAM_SIZE: u64 = 1024 * 1024 * 128;
pub const DRAM_END: u64 = DRAM_SIZE + DRAM_BASE - 1;

pub const CPUID_BASE: u64 = 0x10000;
pub const CPUID_SIZE: u64 = 0x1000;
pub const CPUID_END: u64 = CPUID_BASE + CPUID_SIZE - 1;
