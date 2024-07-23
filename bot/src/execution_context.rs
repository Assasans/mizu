use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use twilight_http::Client;
use twilight_model::id::Id;
use twilight_model::id::marker::ChannelMarker;
use runtime::cpu::Cpu;
use runtime::isolate::Isolate;
use runtime::perf_counter::CPU_TIME_LIMIT;
use crate::{CpuExt, TickResult};

pub struct ExecutionContext {
  pub http: Mutex<Option<Arc<Client>>>,
  pub channel_id: Mutex<Option<Id<ChannelMarker>>>,
  pub isolate: Mutex<Option<Arc<Isolate>>>
}

impl ExecutionContext {
  pub fn new() -> Self {
    Self {
      http: Mutex::new(None),
      channel_id: Mutex::new(None),
      isolate: Mutex::new(None)
    }
  }

  pub async fn run_core(&self, cpu: Arc<Mutex<Cpu>>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let channel_id = self.channel_id.lock().await.unwrap();
    let http = self.http.lock().await.clone().unwrap();

    let wfi = {
      let mut cpu = cpu.lock().await;
      cpu.wfi.clone()
    };
    'wfi: loop {
      wfi.wait_for(|wfi| *wfi == false).await;
      http.create_message(channel_id).content(&format!("wfi: reset"))?.await?;

      let mut cpu = cpu.lock().await;
      loop {
        match cpu.run_tick().await? {
          TickResult::Continue => continue,
          TickResult::Exception(exception) => {
            http.create_message(channel_id).content(&format!("cpu: exception: {}", exception))?.await?;
          }
          TickResult::Eof => {
            http.create_message(channel_id).content(&format!("cpu: execution finished ```c\n{}```", cpu.dump()))?.await?;
          }
          TickResult::Halt => {
            http.create_message(channel_id).content(&format!("cpu: execution halted ```c\n{}```", cpu.dump()))?.await?;
          }
          TickResult::TimeLimit => {
            http.create_message(channel_id).content(&format!("cpu: running too long without yield: `{:?} > {:?}`", cpu.perf.cpu_time, CPU_TIME_LIMIT))?.await?;
          }
          TickResult::WaitForInterrupt => {
            http.create_message(channel_id).content(&format!("wfi: waiting for interrupt at `{:#08x}`", cpu.pc))?.await?;
            continue 'wfi;
          }
        }
        break 'wfi;
      }
    }

    Ok(())
  }
}
