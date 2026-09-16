//! The Ternitor mark: the Node hexagon with a red X badged into its lower
//! corner, on a dark tile.
//!
//! Node's shape, because the window Ternitor hides is almost always a `node`
//! process being handed a console it never asked for; the X is the app's job.
//! The X is knocked back out of the hexagon by a ring of tile, so it reads as a
//! badge without needing a border of its own.
//!
//! **This file is the only definition of the mark.** It is pure geometry over
//! pure `std`, with no Win32 in it, for one reason: `build.rs` pulls it in with
//! `#[path]` to bake the executable's `.ico` from the same arithmetic the tray
//! icon is drawn with. `assets/icon.svg` is generated from the same constants
//! too, and a test fails if the committed SVG drifts from them.

// The `.ico` and `.svg` writers below are for `build.rs` and for the tests. The
// running app only ever calls `render_pixels`, so the rest reads as unused here
// without being unused anywhere.
#![allow(dead_code)]

// ------------------------------------------------------------- the design ---
//
// Unit space: every constant is a fraction of the icon's edge, so one set of
// numbers serves 16px and 256px alike.

/// Corner radius of the tile.
const TILE_R: f64 = 0.225;

/// The hexagon: pointy-top, so its vertices run top then clockwise.
const HEX_CX: f64 = 0.455;
const HEX_CY: f64 = 0.44;
const HEX_R: f64 = 0.335; // circumradius: centre to vertex

/// The X badge. It straddles the hexagon's lower-right edge, so the knock-back
/// bites the corner rather than sitting beside it.
const X_CX: f64 = 0.66;
const X_CY: f64 = 0.645;
const X_ARM: f64 = 0.132; // half the badge's extent
const X_HALF: f64 = 0.053; // half the stroke width
const X_GAP: f64 = 0.027; // tile showing between hexagon and X

/// Tile, top to bottom. Darker than a taskbar in either Windows theme, so the
/// silhouette holds, but not pure black: the hexagon and the X are the only
/// colour on it.
const TILE_TOP: Rgb = (0x1B as f64, 0x1F as f64, 0x21 as f64);
const TILE_BOTTOM: Rgb = (0x09 as f64, 0x0B as f64, 0x0C as f64);

/// Node green, light at the top-left of the hexagon to dark at the bottom-right.
const HEX_LIGHT: Rgb = (106.0, 191.0, 75.0);
const HEX_DARK: Rgb = (63.0, 135.0, 63.0);

const X_RED: Rgb = (220.0, 38.0, 38.0);

pub type Rgb = (f64, f64, f64);

/// The sizes baked into the `.ico`. 256 is by far the largest entry -- it is
/// what Explorer's extra-large view asks for, and it is why the file is ~370 KB.
pub const ICO_SIZES: [i32; 9] = [16, 20, 24, 32, 40, 48, 64, 128, 256];

// ---------------------------------------------------------------- geometry ---

/// Is `p` within `half_w` of the segment `a`-`b`? A capsule, which is what a
/// round-capped stroke of that width covers.
fn near_segment(p: (f64, f64), a: (f64, f64), b: (f64, f64), half_w: f64) -> bool {
    let (vx, vy) = (b.0 - a.0, b.1 - a.1);
    let len2 = vx * vx + vy * vy;
    let t = if len2 <= 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * vx + (p.1 - a.1) * vy) / len2).clamp(0.0, 1.0)
    };
    let (dx, dy) = (p.0 - (a.0 + t * vx), p.1 - (a.1 + t * vy));
    dx * dx + dy * dy <= half_w * half_w
}

/// Pointy-top hexagon, sized by the icon's edge. Vertices run clockwise from the
/// top so the inside test can be a single consistent turn direction.
fn hexagon_vertices(s: f64) -> [(f64, f64); 6] {
    let r = HEX_R * s;
    let (hx, hy) = (HEX_CX * s, HEX_CY * s);
    let mut v = [(0.0, 0.0); 6];
    for (k, slot) in v.iter_mut().enumerate() {
        let a = std::f64::consts::FRAC_PI_2 - k as f64 * std::f64::consts::PI / 3.0;
        *slot = (hx + r * a.cos(), hy - r * a.sin());
    }
    v
}

fn inside_hexagon(v: &[(f64, f64); 6], p: (f64, f64)) -> bool {
    for k in 0..6 {
        let (x1, y1) = v[k];
        let (x2, y2) = v[(k + 1) % 6];
        let (ax, ay) = (x2 - x1, y2 - y1);
        if ax * (p.1 - y1) - ay * (p.0 - x1) <= 0.0 {
            return false;
        }
    }
    true
}

/// The rounded tile: clamp into the inner rect, then ask whether the point is
/// within the corner radius of where it landed.
fn inside_tile(s: f64, p: (f64, f64)) -> bool {
    let r = TILE_R * s;
    let cx = p.0.clamp(r, s - r);
    let cy = p.1.clamp(r, s - r);
    let (dx, dy) = (p.0 - cx, p.1 - cy);
    dx * dx + dy * dy <= r * r
}

/// The X badge's two strokes, at a given half-width. Both the red X and its
/// knock-back are this shape at different widths.
fn on_x(s: f64, p: (f64, f64), half_w: f64) -> bool {
    let x0 = (X_CX - X_ARM) * s;
    let x1 = (X_CX + X_ARM) * s;
    let y0 = (X_CY - X_ARM) * s;
    let y1 = (X_CY + X_ARM) * s;
    near_segment(p, (x0, y0), (x1, y1), half_w) || near_segment(p, (x0, y1), (x1, y0), half_w)
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

fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    (
        a.0 + (b.0 - a.0) * t,
        a.1 + (b.1 - a.1) * t,
        a.2 + (b.2 - a.2) * t,
    )
}

/// The hexagon's bounding box, and the extent of the diagonal the green
/// gradient runs along. Shared with `svg` so the two renderings grade the same.
fn hexagon_box(s: f64) -> (f64, f64, f64, f64) {
    let w = 3f64.sqrt() * HEX_R * s; // flat-to-flat width of a pointy-top hexagon
    let h = 2.0 * HEX_R * s;
    (HEX_CX * s - w / 2.0, HEX_CY * s - h / 2.0, w, h)
}

// -------------------------------------------------------------- the raster ---

/// The mark as a `size` x `size` BGRA buffer, ready for `CreateIcon` or for an
/// `.ico` entry.
pub fn render_pixels(size: i32) -> Vec<u8> {
    let s = size as f64;
    let v = hexagon_vertices(s);
    let (bx, by, bw, bh) = hexagon_box(s);

    let mut out = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        let ty = (y as f64 + 0.5) / s;
        for x in 0..size {
            // Outside the tile is transparent, and nothing else is drawn there.
            let a = cover(x, y, &|px, py| inside_tile(s, (px, py)));
            if a <= 0.0 {
                continue;
            }

            let base = mix(TILE_TOP, TILE_BOTTOM, ty);
            let mut c = base;

            let h = cover(x, y, &|px, py| inside_hexagon(&v, (px, py))) * a;
            if h > 0.0 {
                // Down the diagonal of the hexagon's own box, so the grade spans
                // the shape rather than the tile.
                let t = ((x as f64 + 0.5 - bx) + (y as f64 + 0.5 - by)) / (bw + bh);
                c = mix(c, mix(HEX_LIGHT, HEX_DARK, t.clamp(0.0, 1.0)), h);
            }

            // The knock-back: tile again, over the hexagon, in the X's shape.
            let g = cover(x, y, &|px, py| on_x(s, (px, py), (X_HALF + X_GAP) * s)) * a;
            if g > 0.0 {
                c = mix(c, base, g);
            }

            let k = cover(x, y, &|px, py| on_x(s, (px, py), X_HALF * s)) * a;
            if k > 0.0 {
                c = mix(c, X_RED, k);
            }

            let o = ((y * size + x) * 4) as usize;
            out[o] = c.2.round() as u8;
            out[o + 1] = c.1.round() as u8;
            out[o + 2] = c.0.round() as u8;
            out[o + 3] = (a * 255.0).round() as u8;
        }
    }
    out
}

// ----------------------------------------------------------------- the ico ---

/// Every size, as a single `.ico` file. Written by hand: the format is a
/// directory of DIBs, and a build script may not pull in a crate to write 40
/// bytes of header.
pub fn ico() -> Vec<u8> {
    let images: Vec<Vec<u8>> = ICO_SIZES.iter().map(|&s| dib(s)).collect();

    let mut out = Vec::new();
    push_u16(&mut out, 0); // reserved
    push_u16(&mut out, 1); // 1: icon
    push_u16(&mut out, images.len() as u16);

    let mut offset = 6 + 16 * images.len() as u32;
    for (&size, img) in ICO_SIZES.iter().zip(&images) {
        // 256 is written as 0 in the one-byte field; that is the format's way.
        out.push(if size >= 256 { 0 } else { size as u8 });
        out.push(if size >= 256 { 0 } else { size as u8 });
        out.push(0); // palette size
        out.push(0); // reserved
        push_u16(&mut out, 1); // colour planes
        push_u16(&mut out, 32); // bits per pixel
        push_u32(&mut out, img.len() as u32);
        push_u32(&mut out, offset);
        offset += img.len() as u32;
    }
    for img in &images {
        out.extend_from_slice(img);
    }
    out
}

/// One icon image: a 32bpp bottom-up DIB with an empty AND mask, which is what
/// an `.ico` entry holds. The alpha channel is what makes the tile's corners
/// transparent; the mask is vestigial and Windows still expects its bytes.
fn dib(size: i32) -> Vec<u8> {
    let px = render_pixels(size);
    let row = (size * 4) as usize;
    let mask_row = (size as usize).div_ceil(32) * 4;
    let mut out = Vec::with_capacity(40 + row * size as usize + mask_row * size as usize);

    push_u32(&mut out, 40); // biSize: BITMAPINFOHEADER
    push_u32(&mut out, size as u32);
    push_u32(&mut out, (size * 2) as u32); // XOR image then AND mask
    push_u16(&mut out, 1);
    push_u16(&mut out, 32);
    push_u32(&mut out, 0); // BI_RGB
    push_u32(&mut out, (row * size as usize) as u32);
    for _ in 0..4 {
        push_u32(&mut out, 0); // resolution and palette counts
    }

    for y in (0..size).rev() {
        let start = y as usize * row;
        out.extend_from_slice(&px[start..start + row]);
    }
    out.resize(out.len() + mask_row * size as usize, 0);
    out
}

fn push_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

// ----------------------------------------------------------------- the svg ---

/// The mark as SVG, at the same proportions the raster uses. Generated, not
/// hand-written: it is another view of these constants, and the test below
/// fails if the committed copy stops matching them.
pub fn svg() -> String {
    let s = 256.0;
    let (bx, by, bw, bh) = hexagon_box(s);
    // The gradient axis that reproduces the raster's diagonal exactly.
    let d = (bw + bh) / 2.0;

    let points: Vec<String> = hexagon_vertices(s)
        .iter()
        .map(|(x, y)| format!("{x:.2},{y:.2}"))
        .collect();

    let x0 = (X_CX - X_ARM) * s;
    let x1 = (X_CX + X_ARM) * s;
    let y0 = (X_CY - X_ARM) * s;
    let y1 = (X_CY + X_ARM) * s;
    let strokes = format!("M{x0:.2} {y0:.2}L{x1:.2} {y1:.2}M{x0:.2} {y1:.2}L{x1:.2} {y0:.2}");

    let (tile_top, tile_bottom) = (hex(TILE_TOP), hex(TILE_BOTTOM));
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {s} {s}" width="{s}" height="{s}" role="img" aria-labelledby="t">
  <title id="t">Ternitor</title>
  <defs>
    <linearGradient id="tile" x1="0" y1="0" x2="0" y2="{s}" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="{tile_top}"/>
      <stop offset="1" stop-color="{tile_bottom}"/>
    </linearGradient>
    <linearGradient id="node" x1="{bx:.2}" y1="{by:.2}" x2="{bx2:.2}" y2="{by2:.2}" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="{hex_light}"/>
      <stop offset="1" stop-color="{hex_dark}"/>
    </linearGradient>
    <mask id="knock">
      <rect width="{s}" height="{s}" fill="#fff"/>
      <path d="{strokes}" stroke="#000" stroke-width="{gap_w:.2}" stroke-linecap="round" fill="none"/>
    </mask>
  </defs>
  <rect width="{s}" height="{s}" rx="{radius:.2}" fill="url(#tile)"/>
  <polygon points="{points}" fill="url(#node)" mask="url(#knock)"/>
  <path d="{strokes}" stroke="{x_red}" stroke-width="{x_w:.2}" stroke-linecap="round" fill="none"/>
</svg>
"##,
        bx2 = bx + d,
        by2 = by + d,
        points = points.join(" "),
        radius = TILE_R * s,
        gap_w = 2.0 * (X_HALF + X_GAP) * s,
        x_w = 2.0 * X_HALF * s,
        hex_light = hex(HEX_LIGHT),
        hex_dark = hex(HEX_DARK),
        x_red = hex(X_RED),
    )
}

fn hex(c: Rgb) -> String {
    format!("#{:02X}{:02X}{:02X}", c.0 as u8, c.1 as u8, c.2 as u8)
}

// ------------------------------------------------------------------ tests ---

#[cfg(test)]
mod tests {
    use super::*;

    const SVG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icon.svg");

    fn at(size: i32, x: i32, y: i32) -> [u8; 4] {
        let px = render_pixels(size);
        let o = ((y * size + x) * 4) as usize;
        [px[o], px[o + 1], px[o + 2], px[o + 3]]
    }

    #[test]
    fn the_hexagon_is_drawn_and_the_corners_are_empty() {
        // Up and left of the badge, which is where the hexagon is untouched.
        let green = at(32, 11, 11);
        assert_eq!(green[3], 255, "the tile is opaque: {green:?}");
        assert!(
            green[1] > green[0] && green[1] > green[2],
            "the hexagon should be green here: {green:?}"
        );
        assert_eq!(at(32, 0, 0)[3], 0, "top-left corner is outside the tile");
        assert_eq!(at(32, 0, 31)[3], 0, "bottom-left corner is outside the tile");
    }

    /// The badge is the point of the mark, so it is worth a pixel: the X sits
    /// over the hexagon, and the knock-back is what keeps it legible there.
    #[test]
    fn the_x_is_red_over_the_hexagon() {
        let badge = at(32, (X_CX * 32.0) as i32, (X_CY * 32.0) as i32);
        assert!(
            badge[2] > 150 && badge[1] < 90 && badge[3] == 255,
            "the badge centre should be red: {badge:?}"
        );
    }

    /// Every size in the `.ico` has to be a real image, not a header.
    #[test]
    fn the_ico_holds_every_size_it_claims() {
        let ico = ico();
        let count = u16::from_le_bytes([ico[4], ico[5]]) as usize;
        assert_eq!(count, ICO_SIZES.len());
        for (i, &size) in ICO_SIZES.iter().enumerate() {
            let entry = 6 + i * 16;
            let w = if ico[entry] == 0 { 256 } else { ico[entry] as i32 };
            assert_eq!(w, size, "entry {i} is the wrong size");
            let len = u32::from_le_bytes(ico[entry + 8..entry + 12].try_into().unwrap()) as usize;
            // BITMAPINFOHEADER + the XOR image + a 1bpp AND mask on 4-byte rows.
            let expect = 40
                + size as usize * size as usize * 4
                + (size as usize).div_ceil(32) * 4 * size as usize;
            assert_eq!(len, expect, "entry {i} is {len} bytes, not {expect}");
        }
    }

    /// The SVG is another view of these constants rather than a second copy of
    /// the design, and this is what keeps it that way.
    #[test]
    fn the_committed_svg_matches_the_geometry() {
        let want = std::fs::read_to_string(SVG).expect("assets/icon.svg is missing");
        let got = svg();
        if want != got {
            let at = want
                .bytes()
                .zip(got.bytes())
                .position(|(a, b)| a != b)
                .unwrap_or(want.len().min(got.len()));
            panic!(
                "assets/icon.svg has drifted from the geometry at byte {at}; regenerate it with `cargo test -- --ignored write_icon_svg`"
            );
        }
    }

    /// Regenerate the committed SVG. Not part of a normal run: it writes into the
    /// source tree, which a test should only do when asked.
    #[test]
    #[ignore = "writes assets/icon.svg"]
    fn write_icon_svg() {
        std::fs::write(SVG, svg()).expect("write assets/icon.svg");
    }
}
