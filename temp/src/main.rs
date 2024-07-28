#![feature(lang_items)]
#![feature(naked_functions)]

#![no_std]
#![no_main]

mod prelude;

use prelude::*;

#[macro_use]
extern crate alloc;

use mizu_hal::discord::*;
use mizu_hal::discord::discord::*;
use mizu_hal::discord::discord::discord_ex_request::DiscordExRequestUnion;
use mizu_hal::discord::prost::alloc::string::ToString;

#[link_section = ".start"]
#[no_mangle]
pub unsafe extern "C" fn _start() {
  let id = __discord_ex(DiscordExRequestUnion::CreateMessageRequest(CreateMessageRequest {
    channel_id: 1173644182062116956,
    reference_id: Some(1265732886338736242),
    content: Some("иди нахуй".to_string()),
    attachments: vec![
      CreateAttachment { name: "amongus.txt".to_string(), data: "Hello, 水の世界！".into() }
    ],
    ..Default::default()
  })) as u64;
  halt();
}
