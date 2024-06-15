use core::num::NonZeroU64;

use crate::StringPtr;

pub mod action {
  pub const EVENT_MASK: u64 = 1 << 32;
  pub const EVENT_MESSAGE_CREATE: u64 = EVENT_MASK | 1;
  pub const EVENT_REACTION_ADD: u64 = EVENT_MASK | 2;

  pub const CREATE_MESSAGE: u64 = 1;
  pub const CREATE_REACTION: u64 = 2;
}

#[repr(C)]
#[derive(Debug)]
pub struct discord_create_message_t {
  pub channel_id: u64,
  pub flags: u64,
  pub reply: Option<NonZeroU64>,
  pub stickers: [Option<NonZeroU64>; 3],
  pub content: StringPtr,
}

#[repr(C)]
#[derive(Debug)]
pub struct discord_create_reaction_t {
  pub channel_id: u64,
  pub message_id: u64,
  pub emoji: StringPtr,
}

#[repr(C)]
#[derive(Debug)]
pub struct discord_message_t {
  pub id: u64,
  pub channel_id: u64,
  pub author_id: u64,
  pub content: StringPtr,
}


#[repr(C)]
#[derive(Debug)]
pub struct discord_event_add_reaction_t {
  pub channel_id: u64,
  pub message_id: u64,
  pub user_id: u64,
  pub emoji: StringPtr,
}
