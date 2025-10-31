#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> u8;
}

#[cfg(target_os = "macos")]
pub fn ax_trusted() -> bool {
    unsafe { AXIsProcessTrusted() != 0 }
}
