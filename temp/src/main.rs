#![feature(lang_items)]
#![feature(naked_functions)]

#![no_std]
#![no_main]

mod prelude;

use prelude::*;

#[macro_use]
extern crate alloc;

use alloc::vec::Vec;
use embedded_graphics::{
  framebuffer,
  framebuffer::{Framebuffer, buffer_size},
  mono_font::{jis_x0201::FONT_9X15, MonoTextStyle},
  pixelcolor::{*, raw::BigEndian},
  prelude::*,
  primitives::PrimitiveStyle,
  text::{Alignment, Text},
};
use jpeg_encoder::{Encoder, ColorType};
use mizu_hal::discord::*;
use mizu_hal::discord::discord::*;
use mizu_hal::discord::discord::discord_ex_request::DiscordExRequestUnion;
use mizu_hal::discord::prost::alloc::string::ToString;

#[link_section = ".start"]
#[no_mangle]
pub unsafe extern "C" fn _start() {
  __set_power_state(POWERSTATE_BYPASS);
  main();
  halt();
}

pub unsafe fn main() {
  let mut fb = Framebuffer::<Rgb888, _, BigEndian, 256, 48, {buffer_size::<Rgb888>(256, 48)}>::new();

  fb.bounding_box()
    .into_styled(PrimitiveStyle::with_fill(Rgb888::BLACK))
    .draw(&mut fb)
    .unwrap();
  let character_style = MonoTextStyle::new(&FONT_9X15, Rgb888::RED);

  let text = "ｹﾈｼﾝ ｰ ｢AMONGUS｣｡";
  Text::with_alignment(
    text,
    fb.bounding_box().center() + Point::new(0, 15),
    character_style,
    Alignment::Center,
  )
  .draw(&mut fb).unwrap();
  // println!("{:?}", fb.data().len());

  __discord_ex(DiscordExRequestUnion::CreateMessageRequest(CreateMessageRequest {
    channel_id: 1173644182062116956,
    reference_id: None,
    content: Some("ｹﾈｼﾝ ｰ ｢AMONGUS｣｡".to_string()),
    attachments: vec![
      CreateAttachment { name: "image.jpg".to_string(), data: png::encode(fb.data(), fb.size().width as u16, fb.size().height as u16).into() }
    ],
    ..Default::default()
  })) as u64;
}
