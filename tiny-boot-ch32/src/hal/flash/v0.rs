use core::sync::atomic::{Ordering, fence};

pub(crate) const KEY1: u32 = 0x4567_0123;
pub(crate) const KEY2: u32 = 0xCDEF_89AB;

/// The FPEC on flash_v0 chips requires 0x0800_0000-based addresses for
/// programming, even though flash is mapped at 0x0000_0000 for reads.
pub(crate) const FLASH_PROGRAM_BASE: u32 = 0x0800_0000;

/// Option bytes base address.
pub(crate) const OB_BASE: u32 = 0x1FFF_F800;

/// Option bytes size (the meaningful portion is 16 bytes / 4 words,
/// but the FPEC page is 256 bytes).
pub(crate) const OB_SIZE: usize = 16;

// --- Lock / Unlock ---

pub(crate) fn unlock(regs: &ch32_metapac::flash::Flash) {
    regs.keyr().write(|w| w.set_keyr(KEY1));
    fence(Ordering::SeqCst);
    regs.keyr().write(|w| w.set_keyr(KEY2));
    fence(Ordering::SeqCst);

    regs.modekeyr().write(|w| w.set_modekeyr(KEY1));
    fence(Ordering::SeqCst);
    regs.modekeyr().write(|w| w.set_modekeyr(KEY2));
    fence(Ordering::SeqCst);
}

pub(crate) fn unlock_ob(regs: &ch32_metapac::flash::Flash) {
    unlock(regs);

    regs.obkeyr().write(|w| w.set_optkey(KEY1));
    fence(Ordering::SeqCst);
    regs.obkeyr().write(|w| w.set_optkey(KEY2));
    fence(Ordering::SeqCst);

    regs.ctlr().modify(|w| w.set_obwre(true));
    fence(Ordering::SeqCst);
}

pub(crate) fn lock_ob(regs: &ch32_metapac::flash::Flash) {
    regs.ctlr().modify(|w| w.set_obwre(false));
    lock(regs);
}

pub(crate) fn lock(regs: &ch32_metapac::flash::Flash) {
    regs.ctlr().modify(|w| {
        w.set_lock(true);
        w.set_flock(true);
    });
}

// --- Status ---

pub(crate) fn wait_busy(regs: &ch32_metapac::flash::Flash) {
    while regs.statr().read().bsy() {}
}

/// Returns true if a write protection error occurred.
pub(crate) fn check_wrprterr(regs: &ch32_metapac::flash::Flash) -> bool {
    let statr = regs.statr().read();
    if statr.wrprterr() {
        regs.statr().modify(|w| w.set_wrprterr(true));
        return true;
    }
    if statr.eop() {
        regs.statr().modify(|w| w.set_eop(true));
    }
    false
}

// --- Flash page operations ---

/// Erase a single 1KB flash page.
pub(crate) fn erase_page(regs: &ch32_metapac::flash::Flash, addr: u32) {
    regs.ctlr().modify(|w| w.set_page_er(true));
    fence(Ordering::SeqCst);
    regs.addr().write(|w| w.set_addr(FLASH_PROGRAM_BASE + addr));
    fence(Ordering::SeqCst);
    regs.ctlr().modify(|w| w.set_strt(true));
    wait_busy(regs);
    regs.ctlr().modify(|w| w.set_page_er(false));
}

/// Write up to 64 bytes using fast page programming (FTPG).
/// `addr` is the absolute flash address. `data` length must be a multiple of 4.
/// Remaining bytes in the 256-byte FTPG buffer are left at reset default (0xFF).
pub(crate) fn write_page(regs: &ch32_metapac::flash::Flash, addr: u32, data: &[u8]) {
    let prog_addr = FLASH_PROGRAM_BASE + addr;

    // Buffer reset
    regs.ctlr().modify(|w| w.set_page_pg(true));
    regs.ctlr().modify(|w| w.set_bufrst(true));
    wait_busy(regs);
    regs.ctlr().modify(|w| w.set_page_pg(false));

    // Load words into the page buffer
    let mut ptr = prog_addr as *mut u32;
    for chunk in data.chunks_exact(4) {
        let word = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        regs.ctlr().modify(|w| w.set_page_pg(true));
        unsafe { core::ptr::write_volatile(ptr, word) };
        regs.ctlr().modify(|w| w.set_bufload(true));
        wait_busy(regs);
        regs.ctlr().modify(|w| w.set_page_pg(false));
        ptr = unsafe { ptr.add(1) };
    }

    // Commit: set address and start programming
    regs.ctlr().modify(|w| w.set_page_pg(true));
    regs.addr().write(|w| w.set_addr(prog_addr));
    regs.ctlr().modify(|w| w.set_strt(true));
    wait_busy(regs);
    regs.ctlr().modify(|w| w.set_page_pg(false));
}

// --- Option byte operations ---

/// Erase the entire 256-byte option byte page (OBER).
pub(crate) fn erase_ob(regs: &ch32_metapac::flash::Flash) {
    regs.ctlr().modify(|w| w.set_ober(true));
    fence(Ordering::SeqCst);
    regs.ctlr().modify(|w| w.set_strt(true));
    wait_busy(regs);
    regs.ctlr().modify(|w| w.set_ober(false));
}

/// Program all 16 option bytes using FTPG.
/// `words` contains 4 words covering the full OB layout:
///   [0]: RDPR+nRDPR, USER+nUSER
///   [1]: DATA0+nDATA0, DATA1+nDATA1
///   [2]: WRPR0+nWRPR0, WRPR1+nWRPR1
///   [3]: WRPR2+nWRPR2, WRPR3+nWRPR3
pub(crate) fn write_ob(regs: &ch32_metapac::flash::Flash, words: &[u32; 4]) {
    let data: [u8; 16] = unsafe { core::mem::transmute(*words) };
    write_page(regs, OB_BASE, &data);
}
