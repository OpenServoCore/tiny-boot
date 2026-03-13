#![no_std]

pub(crate) mod common;
pub(crate) mod hal;

#[cfg(feature = "bootloader")]
pub mod boot;

#[cfg(feature = "app")]
pub mod app;
