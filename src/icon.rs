//! The Ternitor mark, drawn arithmetically into a BGRA buffer.
//!
//! Ported pixel-for-pixel from the PowerShell implementation so the tray icon
//! does not change when the app does. Node's mark is a pointy-top hexagon; the
//! red X is badged into the lower-right corner and the hexagon is knocked back
//! around it by a thin transparent gap, so the X reads without needing a border.
//! No tile, no frame: the marks are drawn as large as the bitmap allows.
//!
//! Every shape is supersampled 4x4 rather than antialiased analytically, and no
//! drawing library is involved -- just arithmetic into a buffer.

use windows::Win32::UI::WindowsAndMessaging::{CreateIcon, DestroyIcon, HICON};

use crate::win::instance;

/// Which of the two marks a sample point falls inside.
fn near_segment(px: f64, py: f64, x1: f64, y1: f64, x2: f64, y2: f64, half_w: f64) -> bool {
    let vx = x2 - x1;
    let vy = y2 - y1;
    let len2 = vx * vx + vy * vy;
    let mut t = if len2 <= 0.0 {
        0.0
    } else {
        ((px - x1) * vx + (py - y1) * vy) / len2
    };
    if t < 0.0 {
        t = 0.0;
    } else if t > 1.0 {
        t = 1.0;
    }
    let dx = px - (x1 + t * vx);
    let dy = py - (y1 + t * vy);
    dx * dx + dy * dy <= half_w * half_w
}

/// Pointy-top hexagon, sized by height. Vertices run clockwise from the top so
/// the inside test can be a single consistent turn direction.
fn hexagon_vertices(s: f64) -> [(f64, f64); 6] {
    let r = 0.47 * s; // circumradius, centre to vertex
    let (hx, hy) = (0.48 * s, 0.48 * s);
    let mut v = [(0.0, 0.0); 6];
    for (k, slot) in v.iter_mut().enumerate() {
        let a = std::f64::consts::FRAC_PI_2 - k as f64 * std::f64::consts::PI / 3.0;
        *slot = (hx + r * a.cos(), hy - r * a.sin());
    }
    v
}

fn inside_hexagon(v: &[(f64, f64); 6], px: f64, py: f64) -> bool {
    for k in 0..6 {
        let (x1, y1) = v[k];
        let (x2, y2) = v[(k + 1) % 6];
        let (ax, ay) = (x2 - x1, y2 - y1);
        if ax * (py - y1) - ay * (px - x1) <= 0.0 {
            return false;
        }
    }
    true
}

/// The X badge's two strokes, at a given half-width.
fn on_x(s: f64, px: f64, py: f64, half_w: f64) -> bool {
    let a0 = 0.675 * s;
    let a1 = 0.925 * s;
    near_segment(px, py, a0, a0, a1, a1, half_w) || near_segment(px, py, a0, a1, a1, a0, half_w)
}

/// 4x4 supersampled coverage of one pixel by one shape.
fn cover(x: i32, y: i32, inside: &dyn Fn(f64, f64) -> bool) -> f64 {
    const N: i32 = 4;
    let mut hit = 0;
    for sy in 0..N {
        for sx in 0..N {
            let px = x as f64 + (sx as f64 + 0.5) / N as f64;
            let py = y as f64 + (sy as f64 + 0.5) / N as f64;
            if inside(px, py) {
                hit += 1;
            }
        }
    }
    hit as f64 / (N * N) as f64
}

/// The mark as a `size` x `size` BGRA buffer, ready for `CreateIcon`.
pub fn render_pixels(size: i32) -> Vec<u8> {
    let s = size as f64;
    let v = hexagon_vertices(s);

    let green = |px: f64, py: f64| -> bool {
        if !inside_hexagon(&v, px, py) {
            return false;
        }
        // Knock the hexagon back around the X so the X is not lost against it.
        !on_x(s, px, py, 0.050 * s + 0.035 * s)
    };

    let mut px = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let w = cover(x, y, &green);
            let r = cover(x, y, &|px, py| on_x(s, px, py, 0.050 * s));

            // Node green, light at the top-left to dark at the bottom-right.
            let t = (x + y) as f64 / (2.0 * s);
            let (mut cr, mut cg, mut cb) = ((106.0 - 43.0 * t) * w, (191.0 - 56.0 * t) * w, (75.0 - 12.0 * t) * w);
            let mut a = w;

            // The X, composited source-over.
            cr = 220.0 * r + cr * (1.0 - r);
            cg = 38.0 * r + cg * (1.0 - r);
            cb = 38.0 * r + cb * (1.0 - r);
            a = r + a * (1.0 - r);
            if a <= 0.0 {
                continue;
            }

            let o = ((y * size + x) * 4) as usize;
            px[o] = cb as u8;
            px[o + 1] = cg as u8;
            px[o + 2] = cr as u8;
            px[o + 3] = (255.0 * a) as u8;
        }
    }
    px
}

/// The mark as a real icon handle, at the size the shell asks for.
///
/// A 32bpp XOR bitmap with a NULL AND mask: the alpha channel in the buffer is
/// what makes the corners transparent.
pub fn hicon(size: i32) -> HICON {
    let size = size.clamp(16, 256);
    let px = render_pixels(size);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The port is only trustworthy if it reproduces the PowerShell icon exactly.
    /// Point `TERNITOR_ICON_REF` at a raw BGRA blob dumped from the original
    /// `RenderPixels` to run the comparison; the test is skipped without it so
    /// the crate stays portable.
    #[test]
    fn matches_the_powershell_reference() {
        let Ok(path) = std::env::var("TERNITOR_ICON_REF") else {
            return;
        };
        let size: i32 = std::env::var("TERNITOR_ICON_REF_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(32);
        let want = std::fs::read(&path).expect("reference blob");
        let got = render_pixels(size);
        assert_eq!(want.len(), got.len(), "reference is not a {size}x{size} BGRA blob");
        let diffs: Vec<usize> = (0..got.len()).filter(|&i| want[i] != got[i]).collect();
        assert!(
            diffs.is_empty(),
            "{} of {} bytes differ, first at {} (want {}, got {})",
            diffs.len(),
            got.len(),
            diffs[0],
            want[diffs[0]],
            got[diffs[0]]
        );
    }

    #[test]
    fn centre_is_green_and_corners_are_empty() {
        let s = 32;
        let px = render_pixels(s);
        let at = |x: i32, y: i32| -> [u8; 4] {
            let o = ((y * s + x) * 4) as usize;
            [px[o], px[o + 1], px[o + 2], px[o + 3]]
        };
        let centre = at(15, 15);
        assert_eq!(centre[3], 255, "centre should be opaque");
        assert!(centre[1] > centre[0] && centre[1] > centre[2], "centre should be green: {centre:?}");
        assert_eq!(at(0, 0)[3], 0, "top-left corner should be transparent");
        assert_eq!(at(0, 31)[3], 0, "bottom-left corner should be transparent");
    }
}
