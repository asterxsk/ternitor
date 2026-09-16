//! The Ternitor mark: the Node hexagon with a red X badged into its lower
//! corner.
//!
//! Node's shape, because the window Ternitor hides is almost always a `node`
//! process being handed a console it never asked for; the X is the app's job.
//! The X is knocked back out of the hexagon by a ring cut clean through it, so
//! the badge reads on a dark taskbar and on a light one without a border of its
//! own -- and without a plate behind the mark, which there is not.
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
//
// The mark has no tile under it, so the geometry carries its own framing: these
// are the numbers for the mark's bounding box -- hexagon plus the badge's reach
// -- scaled and centred to fill the icon with a 0.05 margin on every side. Move
// one and the others have to be recomputed, or the mark drifts off centre inside
// its own icon.

/// The hexagon: pointy-top, so its vertices run top then clockwise.
const HEX_CX: f64 = 0.438;
const HEX_CY: f64 = 0.466;
const HEX_R: f64 = 0.416; // circumradius: centre to vertex

/// The X badge. It straddles the hexagon's lower-right edge, so the knock-back
/// bites the corner rather than sitting beside it.
const X_CX: f64 = 0.692;
const X_CY: f64 = 0.720;
const X_ARM: f64 = 0.164; // half the badge's extent
const X_HALF: f64 = 0.066; // half the stroke width
const X_GAP: f64 = 0.033; // hexagon cut away between the stroke and the rest

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

/// The X badge's two strokes, at a given half-width. Both the red X and the ring
/// cut out of the hexagon are this shape at different widths.
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
///
/// Three layers, composited as coverage rather than painted in order: the green
/// hexagon, the ring cut clean out of it around the badge, and the red X over
/// both. The cut has to be *transparent* rather than repainted, because there is
/// no tile behind the mark to repaint it with -- so the ring subtracts from the
/// hexagon's alpha and the X then goes over what is left.
pub fn render_pixels(size: i32) -> Vec<u8> {
    let s = size as f64;
    let v = hexagon_vertices(s);
    let (bx, by, bw, bh) = hexagon_box(s);
    let cut = (X_HALF + X_GAP) * s;
    let stroke = X_HALF * s;

    let mut out = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            // Down the diagonal of the hexagon's own box, so the grade spans the
            // shape rather than the icon.
            let t = ((x as f64 + 0.5 - bx) + (y as f64 + 0.5 - by)) / (bw + bh);
            let hex_c = mix(HEX_LIGHT, HEX_DARK, t.clamp(0.0, 1.0));

            let hex = cover(x, y, &|px, py| inside_hexagon(&v, (px, py)));
            let ring = cover(x, y, &|px, py| on_x(s, (px, py), cut));
            // The cut comes out of the hexagon, and only out of the hexagon: the
            // ring does not erase the badge it is there to separate.
            let a = hex * (1.0 - ring);
            let k = cover(x, y, &|px, py| on_x(s, (px, py), stroke));

            // The X is the top layer, so the "over" is driven by its coverage:
            // where it is fully opaque the colour is its own, whatever the
            // hexagon underneath was doing.
            let inv = 1.0 - k;
            let ao = k + a * inv;
            if ao <= 0.0 {
                continue;
            }
            let c = (
                (X_RED.0 * k + hex_c.0 * a * inv) / ao,
                (X_RED.1 * k + hex_c.1 * a * inv) / ao,
                (X_RED.2 * k + hex_c.2 * a * inv) / ao,
            );

            let o = ((y * size + x) * 4) as usize;
            out[o] = c.2.round() as u8;
            out[o + 1] = c.1.round() as u8;
            out[o + 2] = c.0.round() as u8;
            out[o + 3] = (ao * 255.0).round() as u8;
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
/// an `.ico` entry holds. The alpha channel is what makes the space around the
/// mark transparent; the mask is vestigial and Windows still expects its bytes.
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
///
/// The knock-back is a `mask` on the hexagon rather than a stroke painted in a
/// background colour, which is what makes the gap transparent in both renderings
/// instead of only in the raster.
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

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {s} {s}" width="{s}" height="{s}" role="img" aria-labelledby="t">
  <title id="t">Ternitor</title>
  <defs>
    <linearGradient id="node" x1="{bx:.2}" y1="{by:.2}" x2="{bx2:.2}" y2="{by2:.2}" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="{hex_light}"/>
      <stop offset="1" stop-color="{hex_dark}"/>
    </linearGradient>
    <mask id="knock">
      <rect width="{s}" height="{s}" fill="#fff"/>
      <path d="{strokes}" stroke="#000" stroke-width="{gap_w:.2}" stroke-linecap="round" fill="none"/>
    </mask>
  </defs>
  <polygon points="{points}" fill="url(#node)" mask="url(#knock)"/>
  <path d="{strokes}" stroke="{x_red}" stroke-width="{x_w:.2}" stroke-linecap="round" fill="none"/>
</svg>
"##,
        bx2 = bx + d,
        by2 = by + d,
        points = points.join(" "),
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

    /// Render once, sample anywhere: the checks below ask for several pixels of
    /// the same image, and rendering is not free.
    fn frame(size: i32) -> impl Fn(i32, i32) -> [u8; 4] {
        let px = render_pixels(size);
        move |x, y| {
            let o = ((y * size + x) * 4) as usize;
            [px[o], px[o + 1], px[o + 2], px[o + 3]]
        }
    }

    /// A pixel far enough inside the hexagon, up and left of the badge, that
    /// neither the badge nor the ring cut around it can reach.
    const GREEN_AT: (f64, f64) = (0.25, 0.28);

    #[test]
    fn the_hexagon_is_drawn_and_the_corners_are_empty() {
        let at = frame(32);
        let x = (GREEN_AT.0 * 32.0) as i32;
        let y = (GREEN_AT.1 * 32.0) as i32;
        let green = at(x, y);
        assert_eq!(green[3], 255, "the hexagon is opaque here: {green:?}");
        assert!(
            green[1] > green[0] && green[1] > green[2],
            "the hexagon should be green here: {green:?}"
        );
        assert_eq!(at(0, 0)[3], 0, "top-left corner is outside the mark");
        assert_eq!(at(0, 31)[3], 0, "bottom-left corner is outside the mark");
        assert_eq!(at(31, 0)[3], 0, "top-right corner is outside the mark");
    }

    /// The badge is the point of the mark, so it is worth a pixel: the X sits
    /// over the hexagon, and the ring cut around it is what keeps it legible.
    #[test]
    fn the_x_is_red_over_the_hexagon() {
        let at = frame(32);
        let badge = at((X_CX * 32.0) as i32, (X_CY * 32.0) as i32);
        assert!(
            badge[2] > 150 && badge[1] < 90 && badge[3] == 255,
            "the badge centre should be red: {badge:?}"
        );
    }

    /// The knock-back removes the hexagon rather than painting over it. With no
    /// tile behind the mark there is nothing else it could be: a gap that was
    /// painted would have to be painted *something*.
    #[test]
    fn the_ring_around_the_badge_is_cut_clean_out() {
        let s = 128;
        let at = frame(s);
        // Between the badge's stroke and the rest of the hexagon: offset from the
        // badge's centre at right angles to the diagonal, so the distance to both
        // strokes is the same and lands inside the ring but outside the stroke.
        let cut = at((0.577 * s as f64) as i32, (0.720 * s as f64) as i32);
        assert_eq!(cut[3], 0, "the ring should be transparent: {cut:?}");
        // And the hexagon a little further out from it is still solid.
        let green = at((GREEN_AT.0 * s as f64) as i32, (GREEN_AT.1 * s as f64) as i32);
        assert_eq!(green[3], 255, "the hexagon beside the ring is opaque: {green:?}");
    }

    /// The mark has no tile, so nothing else frames it: it has to fill the icon
    /// it is drawn in, or it floats small and off-centre. This is the test that
    /// fails when a constant is nudged and the bounding box stops being centred
    /// -- which the constants above cannot be moved without.
    #[test]
    fn the_mark_fills_its_icon_and_sits_centred() {
        let s = 128;
        let px = render_pixels(s);
        let alpha = |x: i32, y: i32| px[((y * s + x) * 4 + 3) as usize];

        let (mut l, mut r, mut t, mut b) = (s, -1, s, -1);
        for y in 0..s {
            for x in 0..s {
                if alpha(x, y) > 0 {
                    l = l.min(x);
                    r = r.max(x);
                    t = t.min(y);
                    b = b.max(y);
                }
            }
        }
        let n = s as f64;
        assert!(r >= 0, "the mark rendered nothing at all");
        for (edge, margin) in [("left", l as f64), ("top", t as f64)] {
            assert!(margin / n < 0.10, "the {edge} edge leaves {margin} of {s} empty");
        }
        for (edge, margin) in [("right", n - r as f64), ("bottom", n - b as f64)] {
            assert!(margin / n < 0.10, "the {edge} edge leaves {margin} of {s} empty");
        }
        let centre_x = (l + r) as f64 / 2.0;
        let centre_y = (t + b) as f64 / 2.0;
        assert!(
            (centre_x - n / 2.0).abs() / n < 0.02,
            "not centred horizontally: centre is {centre_x}, mid is {}",
            n / 2.0
        );
        assert!(
            (centre_y - n / 2.0).abs() / n < 0.02,
            "not centred vertically: centre is {centre_y}, mid is {}",
            n / 2.0
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
