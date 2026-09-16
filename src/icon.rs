//! The mark's Win32 half: `mark`'s pixels as an icon handle.
//!
//! The shape itself lives in `mark`, because `build.rs` bakes the executable's
//! `.ico` from the same arithmetic. What is left here is the handle: a 32bpp XOR
//! bitmap with a NULL AND mask, where the alpha channel in the buffer is what
//! makes everything outside the mark transparent.

use windows::Win32::UI::WindowsAndMessaging::{CreateIcon, DestroyIcon, HICON};

use crate::win::instance;

/// The mark as a real icon handle, at the size the shell asks for.
pub fn hicon(size: i32) -> HICON {
    let size = size.clamp(16, 256);
    let px = crate::mark::render_pixels(size);
    unsafe {
        CreateIcon(
            Some(instance()),
            size,
            size,
            1,
            32,
            std::ptr::null(),
            px.as_ptr(),
        )
        .unwrap_or_default()
    }
}

/// Free an icon this module made.
pub fn destroy(icon: HICON) {
    if !icon.is_invalid() {
        unsafe {
            let _ = DestroyIcon(icon);
        }
    }
}
