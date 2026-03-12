use ch32_metapac::pfic::Pfic;

/// Trigger a full system reset via the PFIC CFGR register.
/// On RV2, write KEYCODE=0xBEEF with RESETSYS=1.
pub(crate) fn system_reset(pfic: &Pfic) -> ! {
    pfic.cfgr().write(|w| {
        w.set_keycode(0xBEEF);
        w.set_resetsys(true);
    });
    loop {}
}
