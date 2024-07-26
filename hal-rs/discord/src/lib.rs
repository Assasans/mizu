#![no_std]

pub use prost;

pub mod discord {
  include!(concat!(env!("OUT_DIR"), "/discord.rs"));
}
