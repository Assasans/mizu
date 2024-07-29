use std::error::Error;
use std::sync::Arc;

use runtime::cpu::Cpu;
use runtime::isolate::Isolate;
use runtime::perf_counter::CPU_TIME_LIMIT;
use tokio::sync::{oneshot, Mutex};
use tracing::{debug, info};
use twilight_http::Client;
use twilight_model::id::marker::ChannelMarker;
use twilight_model::id::Id;

use crate::{CpuExt, TickResult};

pub struct ExecutionContext {
  pub http: Mutex<Option<Arc<Client>>>,
  pub channel_id: Mutex<Option<Id<ChannelMarker>>>,
  pub isolate: Mutex<Option<Arc<Isolate>>>,
}

impl Default for ExecutionContext {
  fn default() -> Self {
    Self::new()
  }
}

impl ExecutionContext {
  pub fn new() -> Self {
    Self {
      http: Mutex::new(None),
      channel_id: Mutex::new(None),
      isolate: Mutex::new(None),
    }
  }

  pub async fn run_core(&self, cpu: Arc<Mutex<Cpu>>, cpu_ready: Option<oneshot::Sender<()>>) -> Result<(), Box<dyn Error + Send + Sync>> {
    debug!("starting core loop");
    let channel_id = self.channel_id.lock().await.unwrap();
    let http = self.http.lock().await.clone().unwrap();

    let (_cpu_id, wfi) = {
      let cpu = cpu.lock().await;
      (cpu.id, cpu.wfi.clone())
    };

    if let Some(cpu_ready) = cpu_ready {
      cpu_ready.send(()).unwrap();
    }

    info!("started core loop");
    'wfi: loop {
      wfi.wait_for(|wfi| !(*wfi)).await;
      // http.create_message(channel_id).content(&format!("cpu {}/wfi: reset", cpu_id))?.await?;

      let mut cpu = cpu.lock().await;
      loop {
        match cpu.run_tick().await? {
          TickResult::Continue => continue,
          TickResult::Exception(exception) => {
            http
              .create_message(channel_id)
              .content(&format!("cpu {}: exception: {} ```c\n{}```", cpu.id, exception, cpu.dump()))?
              .await?;
          }
          TickResult::Eof => {
            http
              .create_message(channel_id)
              .content(&format!("cpu {}: execution finished ```c\n{}```", cpu.id, cpu.dump()))?
              .await?;
          }
          TickResult::Halt => {
            http
              .create_message(channel_id)
              .content(&format!("cpu {}: execution halted ```c\n{}```", cpu.id, cpu.dump()))?
              .await?;
          }
          TickResult::TimeLimit => {
            http
              .create_message(channel_id)
              .content(&format!(
                "cpu {}: running too long without yield: `{:?} > {:?}`",
                cpu.id, cpu.perf.cpu_time, CPU_TIME_LIMIT
              ))?
              .await?;
          }
          TickResult::WaitForInterrupt => {
            http
              .create_message(channel_id)
              .content(&format!("cpu {}/wfi: waiting for interrupt at `{:#08x}`", cpu.id, cpu.pc))?
              .await?;
            continue 'wfi;
          }
        }
        break 'wfi;
      }
    }

    Ok(())
  }
}
