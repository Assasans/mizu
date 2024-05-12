use core::{ffi::c_char, num::NonZeroU64};

pub mod action {
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
  pub content: *const c_char,
}

unsafe impl Send for discord_create_message_t {}

#[repr(C)]
#[derive(Debug)]
pub struct discord_create_reaction_t {
  pub channel_id: u64,
  pub message_id: u64,
  pub emoji: *const c_char,
}

unsafe impl Send for discord_create_reaction_t {}

#[repr(C)]
#[derive(Debug)]
pub struct discord_message_t {
  pub id: u64,
  pub channel_id: u64,
  pub author_id: u64,
  pub content: *const c_char,
}

unsafe impl Send for discord_message_t {}
