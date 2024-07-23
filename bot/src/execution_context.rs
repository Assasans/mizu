use std::sync::Arc;
use tokio::sync::Mutex;
use twilight_http::Client;
use twilight_model::id::Id;
use twilight_model::id::marker::ChannelMarker;
use runtime::isolate::Isolate;

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
}
