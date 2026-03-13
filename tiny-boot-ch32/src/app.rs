use tiny_boot::traits::{BootClient as TBBootClient, BootMeta, BootState};

use crate::common::{BOOT_REQUEST_MAGIC, BOOT_REQUEST_PTR, META_BASE};
use crate::hal::flash::FlashWriter;
use crate::hal::pfic;

const META_PTR: *const BootMeta = META_BASE as *const BootMeta;

/// Byte offset of the `state` field within `BootMeta`.
const STATE_OFFSET: u32 = 0;

/// App-side boot client for CH32 chips.
///
/// Provides boot confirmation and bootloader entry request
/// using the flash and PFIC peripherals.
///
/// All operations are wrapped in a critical section so they
/// are safe to call from interrupt context.
pub struct BootClient {
    flash: ch32_metapac::flash::Flash,
}

impl Default for BootClient {
    fn default() -> Self {
        Self {
            flash: ch32_metapac::FLASH,
        }
    }
}

impl TBBootClient for BootClient {
    fn confirm(&mut self) {
        critical_section::with(|_| {
            let meta: BootMeta = unsafe { core::ptr::read_volatile(META_PTR) };
            if meta.boot_state() != BootState::Validating {
                return;
            }
            let next = meta.state & (meta.state >> 1);
            let writer = FlashWriter::standard(&self.flash);
            writer.write_halfword(META_BASE + STATE_OFFSET, next);
        });
    }

    fn request_update(&mut self) -> ! {
        critical_section::with(|_| {
            unsafe { core::ptr::write_volatile(BOOT_REQUEST_PTR, BOOT_REQUEST_MAGIC) };
        });
        pfic::system_reset(&ch32_metapac::PFIC);
    }
}
