#![no_std]
#![no_main]

use panic_halt as _;

#[cfg(feature = "defmt")]
use defmt_rtt as _;

use tiny_boot_ch32::boot::Bootloader;

#[unsafe(export_name = "main")]
fn main() -> ! {
    Bootloader::default().run();
}
