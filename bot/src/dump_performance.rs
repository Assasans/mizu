use std::sync::Arc;
use async_trait::async_trait;
use runtime::bus::BusMemoryExt;
use runtime::csr::MCAUSE;
use runtime::exception::Exception;
use runtime::interrupt::Interrupt;
use tracing::error;
use twilight_http::Client;
use twilight_model::id::Id;
use twilight_model::id::marker::{ChannelMarker, GuildMarker};
use twilight_standby::Standby;
use runtime::cpu::{Cpu, InterruptHandler};

pub struct DumpPerformanceHandler {
  pub guild_id: Id<GuildMarker>,
  pub channel_id: Id<ChannelMarker>,
  pub standby: Arc<Standby>,
  pub http: Arc<Client>,
}

#[async_trait]
impl InterruptHandler for DumpPerformanceHandler {
  async fn handle(&self, cpu: &mut Cpu) {
    self.http.create_message(self.channel_id)
      .content(&format!("performance dump: ```c\nperf={:?}\npc = 0x{:x}```", cpu.perf, cpu.pc)).unwrap()
      .await.unwrap();
    cpu.perf.reset();

    cpu.regs[10] = 0x50;
    cpu.handle_interrupt(Interrupt::PlatformDefined16);
    loop {
      let inst = match cpu.fetch() {
        Ok(inst) => inst,
        Err(exception) => {
          cpu.handle_exception(exception);
          if let Exception::InstructionAccessFault(0) = &exception {
            break;
          }
          if exception.is_fatal() {
            self.http.create_message(self.channel_id).content(&format!("fetch failed: {:?}", exception)).unwrap().await.unwrap();
            error!("fetch failed: {:?}", exception);
            break;
          }

          break;
        }
      };

      match cpu.execute(inst).await {
        Ok(new_pc) => cpu.pc = new_pc,
        Err(exception) => {
          cpu.handle_exception(exception);
          if exception.is_fatal() {
            self.http.create_message(self.channel_id).content(&format!("execute failed: {:?}", exception)).unwrap().await.unwrap();
            error!("execute failed: {:?}", exception);
            break;
          }
          break;
        }
      };
      cpu.perf.instructions_retired += 1;
      if cpu.halt {
        break;
      }
      if cpu.csr.load(MCAUSE) == 0 {
        error!("exited from trap");
        break; // Exited from trap
      }
    }
    let ptr = cpu.saved_regs[10];
    cpu.saved_regs.fill(0);
    cpu.bus.write_string(ptr, "the fog is coming shit").unwrap();

    self.http.create_message(self.channel_id)
      .content(&format!("allocated: `0x{:x}`", ptr)).unwrap()
      .await.unwrap();
  }
}
