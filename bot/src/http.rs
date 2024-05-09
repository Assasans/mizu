use std::ffi::c_char;
use std::sync::Arc;
use async_trait::async_trait;
use reqwest::Method;
use tracing::debug;
use twilight_http::Client;
use twilight_model::id::Id;
use twilight_model::id::marker::{ChannelMarker, GuildMarker};
use twilight_standby::Standby;
use runtime::bus::BusMemoryExt;
use runtime::cpu::{Cpu, InterruptHandler};
use runtime::param::DRAM_BASE;

pub struct HttpHandler {
  pub guild_id: Id<GuildMarker>,
  pub channel_id: Id<ChannelMarker>,
  pub standby: Arc<Standby>,
  pub http: Arc<Client>,
}

#[repr(C)]
#[derive(Debug)]
pub struct http_request_t {
  pub url: *const c_char,
}

unsafe impl Send for http_request_t {}

#[repr(C)]
#[derive(Debug)]
pub struct http_response_t {
  pub status_code: u16,
  pub body: *const c_char,
}

unsafe impl Send for http_response_t {}

#[async_trait]
impl InterruptHandler for HttpHandler {
  async fn handle(&self, cpu: &mut Cpu) {
    let address = cpu.regs[10];
    let request = cpu.bus.read_struct::<http_request_t>(address).unwrap();
    debug!("request: {:?}", request);

    let url = cpu.bus.read_string(request.url as u64).unwrap().to_string_lossy().to_string();
    debug!("url: {}", url);

    let client = reqwest::Client::new();
    let response = client.request(Method::GET, url).send().await.unwrap();

    let ffi_response = http_response_t {
      status_code: response.status().as_u16(),
      body: (DRAM_BASE + 0x9900) as *const c_char,
    };

    cpu.bus.write_string(ffi_response.body as u64, &response.text().await.unwrap()).unwrap();
    cpu.bus.write_struct(DRAM_BASE + 0x6000, &ffi_response).unwrap();
    cpu.regs[10] = DRAM_BASE + 0x6000;
  }
}
