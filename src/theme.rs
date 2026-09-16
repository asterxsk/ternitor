//! Palette, type and metrics for the cutting-bench sheet.
//!
//! The world is a film cutting bench: black base, white ink, one perforated rail
//! and a grease pencil. Three colours and no others -- the ground, the ink, and
//! the orange the operator marks with. Everything between them is film grey, and
//! nothing on the surface is a colour: state is a mark (a punched hole, a
//! slashed frame), never a hue.
//!
//! The bench is black in every Windows theme; see `PALETTE` for why there is no
//! light variant.

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    ANTIALIASED_QUALITY, CreateFontW, DEFAULT_CHARSET, DeleteObject, FF_DONTCARE, GetDC,
    GetTextFaceW, HFONT, OUT_TT_PRECIS, ReleaseDC, SelectObject,
};

use crate::win::{pcw, wide};

/// A colour in the form the drawing code wants it (COLORREF is 0x00BBGGRR).
pub type Rgb = u32;

const fn rgb(r: u8, g: u8, b: u8) -> Rgb {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

pub struct Palette {
    /// Film base. The whole ground.
    pub ground: Rgb,
    /// The rail the strip runs over, a shade above the base.
    pub band: Rgb,
    /// Grain. One value, scattered, never blended.
    pub speck: Rgb,
    /// Dust and hairs on the print: the few specks that catch light.
    pub dust: Rgb,
    /// One-pixel structure: frame edges, perforation outlines.
    pub edge: Rgb,
    /// Section rules.
    pub rule: Rgb,
    /// Small caps labels and the footer. Kept above 4.5:1 on the ground.
    pub dim: Rgb,
    /// Running copy: the paragraph that says what the app does, and the small
    /// print under a control. A step above `dim` so explanation never reads as
    /// annotation.
    pub body: Rgb,
    /// Primary copy, values, and the punched state of a control.
    pub ink: Rgb,
    /// Grease pencil. The operator's hand, and nothing else.
    pub accent: Rgb,
}

/// The bench, in one place.
///
/// There is no light variant. A film cutting bench is a dark room with a lit
/// viewer, and the whole surface is built around one black ground with ink lifted
/// off it -- inverting it would be a different object, not the same object in
/// another theme. So the sheet is black in every Windows theme, and the caption
/// is tinted to match.
pub const PALETTE: Palette = Palette {
    ground: rgb(0x00, 0x00, 0x00),
    band: rgb(0x0C, 0x0C, 0x0C),
    speck: rgb(0x12, 0x12, 0x12),
    dust: rgb(0x2C, 0x2C, 0x2C),
    edge: rgb(0x2E, 0x2E, 0x2E),
    rule: rgb(0x26, 0x26, 0x26),
    dim: rgb(0x8A, 0x8A, 0x8A),
    body: rgb(0xC0, 0xC0, 0xC0),
    ink: rgb(0xFF, 0xFF, 0xFF),
    accent: rgb(0xFF, 0x6A, 0x13),
};

/// Whether Windows is allowed to animate. When it is not, the cut lands whole
/// instead of being drawn.
pub fn animations_enabled() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{
        SPI_GETCLIENTAREAANIMATION, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
    };

    let mut on = windows::core::BOOL(1);
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some(&mut on as *mut windows::core::BOOL as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    ok.is_ok() && on.as_bool()
}

/// The scale factor for a window's monitor, as a multiplier on 96dpi metrics.
/// Hairlines and type both scale with it, so the bench is the same bench at any
/// display scale rather than a stretched bitmap.
pub fn scale_for(hwnd: HWND) -> f32 {
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 }
}

/// The scale factor of the primary display, for sizing a window before it has a
/// monitor of its own.
pub fn scale_for_system() -> f32 {
    use windows::Win32::UI::HiDpi::GetDpiForSystem;
    let dpi = unsafe { GetDpiForSystem() };
    if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 }
}

/// Round a logical measurement to whole device pixels. Everything on the bench
/// snaps this way, which is what keeps a one-pixel rule exactly one pixel wide
/// instead of blurring across two.
pub fn px(logical: f32, scale: f32) -> i32 {
    (logical * scale).round() as i32
}

pub struct Fonts {
    /// The can label: the app's name, set wide and plain.
    pub title: HFONT,
    /// Edge printing: the condensed caps that run along a film edge.
    pub label: HFONT,
    /// Running copy, set in the same grotesque as the label but at reading size.
    pub body: HFONT,
    /// The dense small print under a control.
    pub small: HFONT,
    /// Frame count and window titles: data, so the terminal face.
    pub mono: HFONT,
    /// The counter's wheels.
    pub counter: HFONT,
}

impl Fonts {
    pub fn destroy(&self) {
        for f in [
            self.title,
            self.label,
            self.body,
            self.small,
            self.mono,
            self.counter,
        ] {
            if !f.is_invalid() {
                unsafe {
                    let _ = DeleteObject(f.into());
                }
            }
        }
    }
}

/// Build the bench's faces at a given scale.
pub fn fonts(scale: f32) -> Fonts {
    // Franklin Gothic is the American industrial grotesque that ended up on film
    // can labels and bench signage; Arial Narrow is the condensed face film edge
    // printing is actually set in. Both ship with Windows.
    let title_face = pick_face(&["Franklin Gothic Medium", "Segoe UI Variable Display", "Segoe UI"]);
    let edge_face = pick_face(&["Arial Narrow", "Segoe UI"]);
    let body_face = pick_face(&["Franklin Gothic Book", "Segoe UI Variable Text", "Segoe UI"]);
    // The terminal face of the audience this app serves, and the one Windows
    // itself ships for terminals. Consolas is the always-present fallback.
    let mono_face = pick_face(&["Cascadia Mono", "Consolas"]);

    Fonts {
        title: make_font(&title_face, 21.0, 400, scale),
        label: make_font(&edge_face, 11.5, 700, scale),
        body: make_font(&body_face, 13.5, 400, scale),
        small: make_font(&edge_face, 12.0, 400, scale),
        mono: make_font(&mono_face, 12.5, 400, scale),
        counter: make_font(&mono_face, 34.0, 600, scale),
    }
}

/// Tracking for the edge-printing face, in logical pixels of extra advance.
pub const LABEL_TRACKING: f32 = 1.4;

fn make_font(face: &str, logical_px: f32, weight: i32, scale: f32) -> HFONT {
    let face = wide(face);
    unsafe {
        CreateFontW(
            -((logical_px * scale).round() as i32),
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_TT_PRECIS,
            Default::default(),
            // Greyscale antialiasing, not ClearType. On a black ground subpixel
            // rendering fringes every glyph with orange and blue, which on a
            // surface whose one permitted colour is a grease mark reads as a bug.
            ANTIALIASED_QUALITY,
            FF_DONTCARE.0 as u32,
            pcw(&face),
        )
    }
}

/// First face in `wanted` that this machine actually has.
///
/// Windows silently substitutes a different face when the requested one is
/// missing, which would quietly turn edge printing into Arial, so the presence
/// check is the point: ask GDI what it really selected.
fn pick_face(wanted: &[&str]) -> String {
    for want in wanted {
        if face_exists(want) {
            return (*want).to_string();
        }
    }
    wanted.last().copied().unwrap_or("Segoe UI").to_string()
}

fn face_exists(name: &str) -> bool {
    let hdc = unsafe { GetDC(None) };
    if hdc.is_invalid() {
        return false;
    }
    let probe = make_font(name, 12.0, 400, 1.0);
    let previous = unsafe { SelectObject(hdc, probe.into()) };
    let mut buf = [0u16; 64];
    // The count includes the terminating null.
    let n = unsafe { GetTextFaceW(hdc, Some(&mut buf)) };
    let actual = if n > 1 {
        String::from_utf16_lossy(&buf[..(n as usize - 1).min(buf.len())])
    } else {
        String::new()
    };
    unsafe {
        SelectObject(hdc, previous);
        let _ = DeleteObject(probe.into());
        ReleaseDC(None, hdc);
    }
    actual.eq_ignore_ascii_case(name)
}
