use std::sync::Arc;
use twilight_http::Client;
use twilight_model::id::Id;
use twilight_model::id::marker::ChannelMarker;
use runtime::isolate::Isolate;

pub struct ExecutionContext {
  pub http: Option<Arc<Client>>,
  pub channel_id: Option<Id<ChannelMarker>>,
  pub isolate: Option<Arc<Isolate>>
}

impl ExecutionContext {
  pub fn new() -> Self {
    let mut this = ExecutionContext {
      http: None,
      channel_id: None,
      isolate: None
    };
    this.init_ivt();
    this
  }

  pub fn init_ivt(&mut self) {

  }
}
