use std::time::SystemTime;

use async_trait::async_trait;
use image::codecs::png::PngEncoder;
use image::{ImageEncoder, RgbImage};
use tracing::debug;
use runtime::bus::BusMemoryExt;
use runtime::cpu::{Cpu, InterruptHandler};
use runtime::memory::HARDWARE_BASE;

pub struct PngHandler {}

#[async_trait]
impl InterruptHandler for PngHandler {
  async fn handle(&self, cpu: &mut Cpu) {
    let length = cpu.regs[10];
    let address = cpu.regs[11];

    // [----][----][yyyy][xxxx]
    let resolution = cpu.regs[12];
    let width = resolution as u16;
    let height = (resolution >> 16) as u16;

    debug!("png call: length={} address=0x{:x} width={} height={}", length, address, width, height);

    let pixels = cpu.bus.read(address, length).unwrap();
    let mut output = Vec::new();
    let encoder = PngEncoder::new(&mut output);
    let image = RgbImage::from_raw(width as u32, height as u32, pixels).unwrap();
    image.write_with_encoder(encoder).unwrap();

    cpu.bus.write(HARDWARE_BASE + 0x16000, &output).unwrap();
    cpu.regs[10] = output.len() as u64;
    cpu.regs[11] = HARDWARE_BASE + 0x16000;
  }
}
