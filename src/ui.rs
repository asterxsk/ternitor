//! The settings window: the app's only surface.
//!
//! What it draws is described in DESIGN.md; this module is only how. Three things
//! about the how are load-bearing:
//!
//! - **Two buffers.** Everything static -- ground, grain, rail, perforations, cell
//!   dividers -- is baked into one bitmap and rebuilt only when the client size or
//!   the display scale changes. Each paint blits that bitmap into a second buffer,
//!   draws the live parts over it, and blits the result to the screen. So a repaint
//!   is a blit plus a dozen primitives, which is what lets the cut animate without
//!   the app ever showing up in Task Manager.
//! - **One thread.** The window, the tray icon and the window-event hook all live
//!   on the main thread (see `app`), so `UI` is a plain thread-local with no locks
//!   and the hook callback can invalidate the window directly.
//! - **Lazy and singular.** The window is created on first `open()` and then only
//!   ever shown or hidden. Closing it returns to the tray rather than quitting;
//!   the tray menu owns quitting.
//!
//! `notify_activity` is called from inside the janitor's callback, which runs
//! while `app::App` is mutably borrowed. It therefore touches nothing but `UI`,
//! and reads the app snapshot only later, during `WM_PAINT`.

use std::cell::RefCell;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR, DWMWA_USE_IMMERSIVE_DARK_MODE,
    DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreatePen, CreateSolidBrush,
    DeleteDC, DeleteObject, DrawTextW, EndPaint, FillRect, GetStockObject, GetTextExtentPoint32W,
    HBITMAP, HBRUSH, HDC, HFONT, HGDIOBJ, InvalidateRect, LineTo, MoveToEx, NULL_BRUSH, PS_SOLID,
    PAINTSTRUCT, Rectangle, RoundRect, SRCCOPY, ScreenToClient, SelectObject, SetBkMode,
    SetTextCharacterExtra, SetTextColor, TRANSPARENT, TextOutW, DT_CENTER, DT_SINGLELINE,
    DT_VCENTER,
};
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    TrackMouseEvent, TRACKMOUSEEVENT, TME_LEAVE, VK_ESCAPE, VK_RETURN, VK_SPACE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, CreateWindowExW, CW_USEDEFAULT, DefWindowProcW, DestroyWindow,
    GetClientRect, GetCursorPos, IDC_ARROW, IDC_HAND, IsWindowVisible, KillTimer, LoadCursorW,
    MINMAXINFO, RegisterClassW, SetCursor, SetForegroundWindow, SetTimer, SetWindowPos, ShowWindow,
    SW_HIDE, SW_SHOWNORMAL, SWP_NOACTIVATE, SWP_NOZORDER, WM_ACTIVATE, WM_CLOSE, WM_DESTROY,
    WM_DPICHANGED, WM_ERASEBKGND, WM_GETMINMAXINFO, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MOUSEMOVE, WM_PAINT, WM_SETFOCUS, WM_SETCURSOR, WM_TIMER, WNDCLASSW, WINDOW_EX_STYLE,
    WINDOW_STYLE, WS_CAPTION, WS_MINIMIZEBOX, WS_SYSMENU,
};

use crate::app::{self, Snapshot};
use crate::theme::{self, Fonts, Palette, Rgb};
use crate::win::{instance, pcw, wide};

/// `Win32::UI::Controls::WM_MOUSELEAVE`, declared here so the crate does not have
/// to pull in that whole feature for one message number.
const WM_MOUSELEAVE: u32 = 0x02A3;

// ---------------------------------------------------------------- metrics ---
//
// Logical pixels at 96dpi. Everything scales through `theme::px`.

const W: i32 = 720;
const H: i32 = 428;
const PAD: i32 = 44;

/// The rail: band height, and the frame cell that hangs in it.
const STRIP_H: i32 = 84;
const CELL_W: i32 = 58;
const CELL_EDGE: i32 = 6;
const CELL_TOP: i32 = 20;
const CELL_BOTTOM: i32 = 72;
/// Slots across the strip. The last one is clipped by the window edge, which is
/// the point: when they are all lit, the oldest frame is visibly leaving.
const CELLS: i32 = 13;

/// The counter's wheels.
const DIGIT_W: i32 = 40;
const DIGIT_GAP: i32 = 3;
const DIGIT_H: i32 = 58;

/// The autostart punch.
const PUNCH_X: i32 = 452;
const PUNCH_Y: i32 = 248;
const PUNCH_W: i32 = 64;
const PUNCH_H: i32 = 36;

/// How long the cut takes, start to finish.
const CUT_MS: u64 = 340;
const FRAME_MS: u32 = 16;
const TIMER_ID: usize = 1;

// ------------------------------------------------------------------ state ---

/// A memory DC with a bitmap the size of the client, kept across paints.
struct Buf {
    dc: HDC,
    bmp: HBITMAP,
    old: HGDIOBJ,
    w: i32,
    h: i32,
}

impl Buf {
    fn new(screen: HDC, w: i32, h: i32) -> Buf {
        unsafe {
            let dc = CreateCompatibleDC(Some(screen));
            let bmp = CreateCompatibleBitmap(screen, w, h);
            let old = SelectObject(dc, bmp.into());
            Buf { dc, bmp, old, w, h }
        }
    }

    fn fits(&self, w: i32, h: i32) -> bool {
        self.w >= w && self.h >= h
    }
}

impl Drop for Buf {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.old);
            let _ = DeleteObject(self.bmp.into());
            let _ = DeleteDC(self.dc);
        }
    }
}

struct Ui {
    hwnd: HWND,
    /// When the most recent cut landed, while it is still being drawn.
    pulse: Option<Instant>,
    ticking: bool,
    hover: bool,
    focus: bool,
    pressed: bool,
    /// Why the last toggle did not take. Shown under the punch, because a control
    /// that refuses silently is worse than no control.
    autostart_error: Option<String>,
    scale: f32,
    fonts: Option<Fonts>,
    /// Ground, grain, rail and perforations, baked. Kept between paints.
    ground: Option<Buf>,
    /// What gets composed and blitted each paint.
    buf: Option<Buf>,
}

impl Ui {
    fn new() -> Ui {
        Ui {
            hwnd: HWND::default(),
            pulse: None,
            ticking: false,
            hover: false,
            focus: false,
            pressed: false,
            autostart_error: None,
            scale: 1.0,
            fonts: None,
            ground: None,
            buf: None,
        }
    }

    /// Drop everything derived from the size, scale or theme.
    fn drop_cache(&mut self) {
        self.ground = None;
        self.buf = None;
        if let Some(f) = self.fonts.take() {
            f.destroy();
        }
    }
}

thread_local! {
    static UI: RefCell<Ui> = RefCell::new(Ui::new());
}

fn with_ui<R>(f: impl FnOnce(&mut Ui) -> R) -> R {
    UI.with(|u| f(&mut u.borrow_mut()))
}

// ------------------------------------------------------------- the bench ---

struct Painter {
    hdc: HDC,
    s: f32,
}

impl Painter {
    fn d(&self, logical: f32) -> i32 {
        theme::px(logical, self.s)
    }

    fn fill(&self, r: RECT, c: Rgb) {
        unsafe {
            let b = CreateSolidBrush(COLORREF(c));
            FillRect(self.hdc, &r, b);
            let _ = DeleteObject(b.into());
        }
    }

    /// A one-pixel outline with no fill.
    fn frame(&self, r: RECT, c: Rgb) {
        self.shape(r, c, None, 0, |hdc, l, t, rr, b| unsafe {
            let _ = Rectangle(hdc, l, t, rr, b);
        });
    }

    fn round(&self, r: RECT, radius: i32, stroke: Rgb, fill: Option<Rgb>) {
        self.shape(r, stroke, fill, radius, |hdc, l, t, rr, b| unsafe {
            let _ = RoundRect(hdc, l, t, rr, b, radius * 2, radius * 2);
        });
    }

    fn shape(
        &self,
        r: RECT,
        stroke: Rgb,
        fill: Option<Rgb>,
        _radius: i32,
        draw: impl FnOnce(HDC, i32, i32, i32, i32),
    ) {
        unsafe {
            let p = CreatePen(PS_SOLID, 1, COLORREF(stroke));
            let b = CreateSolidBrush(COLORREF(fill.unwrap_or(0)));
            let ob = SelectObject(
                self.hdc,
                if fill.is_some() {
                    b.into()
                } else {
                    GetStockObject(NULL_BRUSH)
                },
            );
            let op = SelectObject(self.hdc, p.into());
            draw(self.hdc, r.left, r.top, r.right, r.bottom);
            SelectObject(self.hdc, op);
            SelectObject(self.hdc, ob);
            let _ = DeleteObject(p.into());
            let _ = DeleteObject(b.into());
        }
    }

    fn line(&self, x1: i32, y1: i32, x2: i32, y2: i32, c: Rgb, weight: i32) {
        unsafe {
            let p = CreatePen(PS_SOLID, weight.max(1), COLORREF(c));
            let op = SelectObject(self.hdc, p.into());
            let _ = MoveToEx(self.hdc, x1, y1, None);
            let _ = LineTo(self.hdc, x2, y2);
            SelectObject(self.hdc, op);
            let _ = DeleteObject(p.into());
        }
    }

    fn hrule(&self, y: i32, c: Rgb) {
        self.line(self.d(PAD as f32), y, self.d((W - PAD) as f32), y, c, 1);
    }

    fn with_font<R>(&self, font: HFONT, f: impl FnOnce(HDC) -> R) -> R {
        unsafe {
            let op = SelectObject(self.hdc, font.into());
            SetBkMode(self.hdc, TRANSPARENT);
            let r = f(self.hdc);
            SelectObject(self.hdc, op);
            r
        }
    }

    fn measure(&self, text: &str, font: HFONT, tracking: f32) -> i32 {
        let u = to_utf16(text);
        if u.is_empty() {
            return 0;
        }
        self.with_font(font, |hdc| unsafe {
            let mut size = SIZE::default();
            let _ = GetTextExtentPoint32W(hdc, &u, &mut size);
            size.cx + theme::px(tracking, self.s) * (u.len() as i32 - 1)
        })
    }

    fn text(&self, text: &str, x: i32, y: i32, font: HFONT, c: Rgb, tracking: f32) {
        let u = to_utf16(text);
        if u.is_empty() {
            return;
        }
        self.with_font(font, |hdc| unsafe {
            let oc = SetTextColor(hdc, COLORREF(c));
            SetTextCharacterExtra(hdc, theme::px(tracking, self.s));
            let _ = TextOutW(hdc, x, y, &u);
            SetTextCharacterExtra(hdc, 0);
            SetTextColor(hdc, oc);
        });
    }

    /// Right-aligned text ending at `right`.
    fn text_right(&self, text: &str, right: i32, y: i32, font: HFONT, c: Rgb, tracking: f32) {
        let w = self.measure(text, font, tracking);
        self.text(text, right - w, y, font, c, tracking);
    }

    /// Text centred both ways in `r`.
    fn text_in(&self, text: &str, r: RECT, font: HFONT, c: Rgb) {
        let mut u = to_utf16(text);
        if u.is_empty() {
            return;
        }
        let mut rect = r;
        self.with_font(font, |hdc| unsafe {
            let oc = SetTextColor(hdc, COLORREF(c));
            let _ = DrawTextW(hdc, &mut u, &mut rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
            SetTextColor(hdc, oc);
        });
    }

    /// `text`, shortened with an ellipsis if it will not fit in `max_w`.
    fn fit(&self, text: &str, max_w: i32, font: HFONT, tracking: f32) -> String {
        if self.measure(text, font, tracking) <= max_w {
            return text.to_string();
        }
        let chars: Vec<char> = text.chars().collect();
        let mut n = chars.len();
        while n > 1 {
            n -= 1;
            let candidate: String = chars[..n].iter().collect::<String>() + "\u{2026}";
            if self.measure(&candidate, font, tracking) <= max_w {
                return candidate;
            }
        }
        "\u{2026}".to_string()
    }
}

fn to_utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

/// Ground, grain, dust, the rail, its perforations and the cell dividers. All
/// static, so it is drawn once per size and theme rather than once per frame.
fn bake_ground(p: &Painter, pal: &Palette) {
    let full = RECT {
        left: 0,
        top: 0,
        right: p.d(W as f32),
        bottom: p.d(H as f32),
    };
    p.fill(full, pal.ground);
    p.fill(
        RECT {
            bottom: p.d(STRIP_H as f32),
            ..full
        },
        pal.band,
    );
    // Grain over ground and rail alike: the whole bench is film, so the tooth
    // runs through both. Only the punched holes stay clean.
    grain(p, pal, p.d(W as f32), p.d(H as f32));

    // Perforations, two rows, running off both edges.
    let hole_w = p.d(11.0);
    let hole_h = p.d(7.0);
    let pitch = p.d(22.0);
    let mut x = -p.d(6.0);
    while x < p.d(W as f32) {
        for top in [p.d(8.0), p.d(77.0)] {
            p.round(
                RECT {
                    left: x,
                    top,
                    right: x + hole_w,
                    bottom: top + hole_h,
                },
                p.d(4.0),
                pal.edge,
                Some(pal.ground),
            );
        }
        x += pitch;
    }

    // The rail's own edges, and a divider between every pair of cells.
    let top = p.d(CELL_TOP as f32 - 1.0);
    let bottom = p.d(CELL_BOTTOM as f32 + 1.0);
    p.line(0, top, p.d(W as f32), top, pal.rule, 1);
    p.line(0, bottom, p.d(W as f32), bottom, pal.rule, 1);
    for j in 1..=CELLS {
        let x = p.d((W - CELL_EDGE) as f32) - j * p.d(CELL_W as f32);
        p.line(x, p.d(CELL_TOP as f32), x, p.d(CELL_BOTTOM as f32), pal.edge, 1);
    }
}

/// Film grain and the occasional dust speck, deterministic in (x, y) so the bench
/// does not crawl between repaints.
fn grain(p: &Painter, pal: &Palette, w: i32, h: i32) {
    let mut brush: Option<(Rgb, HBRUSH)> = None;
    for y in 0..h {
        for x in 0..w {
            let n = scatter(x, y, 1);
            if n % 101 >= 3 {
                continue;
            }
            let c = if n % 1301 == 0 { pal.dust } else { pal.speck };
            if brush.map(|(bc, _)| bc) != Some(c) {
                if let Some((_, old)) = brush.take() {
                    unsafe { let _ = DeleteObject(old.into()); }
                }
                brush = Some((c, unsafe { CreateSolidBrush(COLORREF(c)) }));
            }
            if let Some((_, b)) = brush {
                unsafe {
                    FillRect(
                        p.hdc,
                        &RECT {
                            left: x,
                            top: y,
                            right: x + 1,
                            bottom: y + 1,
                        },
                        b,
                    )
                };
            }
        }
    }
    if let Some((_, b)) = brush {
        unsafe { let _ = DeleteObject(b.into()); }
    }
}

fn scatter(x: i32, y: i32, salt: u32) -> u32 {
    let mut h = (x as u32)
        .wrapping_mul(0x1657_6D5B)
        .wrapping_add((y as u32).wrapping_mul(0x27D4_EB2D))
        .wrapping_add(salt.wrapping_mul(0x9E37_79B9));
    h ^= h >> 15;
    h = h.wrapping_mul(0x2545_F491);
    h ^ (h >> 13)
}

/// A cut mark: the operator's hand across a frame. Two strokes, the longer one
/// thin and overshooting, so it reads as waxy and drawn rather than as a vector.
fn grease(p: &Painter, x1: i32, y1: i32, x2: i32, y2: i32, colour: Rgb) {
    p.line(x1, y1, x2, y2, colour, p.d(3.0).max(2));
    let dx = (x2 - x1) / 6;
    let dy = (y2 - y1) / 6;
    p.line(x1 - dx, y1 - dy, x2 + dx, y2 + dy, colour, 1);
}

/// Exponential ease-out: fast out of the gate, settling onto the mark.
fn ease(t: f64) -> f64 {
    if t >= 1.0 {
        1.0
    } else {
        1.0 - (2.0f64).powf(-10.0 * t)
    }
}

fn draw_frames(p: &Painter, pal: &Palette, snap: &Snapshot, ui: &Ui) {
    let lit = snap.hidden.min(CELLS as u64) as i32;
    let right = p.d((W - CELL_EDGE) as f32);
    let step = p.d(CELL_W as f32);

    for slot in 0..lit {
        let r = right - slot * step;
        let cell = RECT {
            left: r - step,
            top: p.d(CELL_TOP as f32),
            right: r,
            bottom: p.d(CELL_BOTTOM as f32),
        };
        let newest = slot == 0;

        // The cut is drawn as it lands: the frame's content opens left to right,
        // then the hand follows behind it. One gesture, one easing.
        let (opened, slash) = if newest && snap.hidden > 0 {
            match ui.pulse {
                Some(t0) => {
                    let t = (t0.elapsed().as_millis() as f64 / CUT_MS as f64).min(1.0);
                    (ease((t / 0.45).min(1.0)), ((t - 0.35) / 0.65).clamp(0.0, 1.0))
                }
                None => (1.0, 1.0),
            }
        } else {
            (1.0, 0.0)
        };

        let inner = RECT {
            left: cell.left + p.d(7.0),
            top: cell.top + p.d(7.0),
            right: cell.right - p.d(7.0),
            bottom: cell.bottom - p.d(7.0),
        };
        let open_to = inner.left + ((inner.right - inner.left) as f64 * opened) as i32;
        if open_to > inner.left {
            // A window with a title bar and nothing under it.
            let bar = p.d(8.0).max(3);
            p.frame(
                RECT {
                    left: inner.left,
                    top: inner.top,
                    right: open_to,
                    bottom: inner.bottom,
                },
                pal.edge,
            );
            if open_to > inner.left + bar {
                p.line(
                    inner.left + p.d(3.0),
                    inner.top + bar,
                    open_to - p.d(3.0),
                    inner.top + bar,
                    pal.edge,
                    1,
                );
            }
        }

        if slash > 0.0 {
            let (ax, ay) = (inner.left + p.d(4.0), inner.bottom - p.d(4.0));
            let (bx, by) = (inner.right - p.d(4.0), inner.top + p.d(4.0));
            grease(
                p,
                ax,
                ay,
                ax + ((bx - ax) as f64 * slash) as i32,
                ay + ((by - ay) as f64 * slash) as i32,
                pal.accent,
            );
        }
    }
}

/// A mechanical counter's wheels, zero-padded to at least three and growing as
/// the count does. `000` is the honest reading for a session that has hidden
/// nothing yet.
fn counter_digits(hidden: u64) -> String {
    let s = hidden.to_string();
    let width = s.len().max(3);
    format!("{hidden:0width$}")
}

fn draw_counter(p: &Painter, pal: &Palette, snap: &Snapshot, fonts: &Fonts) {
    let right = p.d((W - PAD) as f32);
    let digits = counter_digits(snap.hidden);
    let n = digits.len() as i32;
    let dw = p.d(DIGIT_W as f32);
    let gap = p.d(DIGIT_GAP as f32);
    let top = p.d(130.0);
    let bottom = top + p.d(DIGIT_H as f32);

    // Paused, the wheels stop turning. A figure that keeps showing something the
    // app no longer has is the one failure this surface can commit, so the label
    // says why the number is standing still. It is an indicator, not a control.
    if snap.paused {
        let w = p.measure("PAUSED", fonts.label, theme::LABEL_TRACKING);
        p.text(
            "PAUSED",
            right - w,
            p.d(108.0),
            fonts.label,
            pal.body,
            theme::LABEL_TRACKING,
        );
        p.text_right(
            "NOT COUNTING",
            right - w - p.d(12.0),
            p.d(108.0),
            fonts.label,
            pal.dim,
            theme::LABEL_TRACKING,
        );
    } else {
        p.text_right(
            "HIDDEN THIS SESSION",
            right,
            p.d(108.0),
            fonts.label,
            pal.dim,
            theme::LABEL_TRACKING,
        );
    }

    let housing = RECT {
        left: right - (n * dw + (n - 1) * gap),
        top,
        right,
        bottom,
    };
    p.frame(housing, pal.edge);

    for (i, ch) in digits.chars().enumerate() {
        let cell = RECT {
            left: housing.left + i as i32 * (dw + gap),
            top,
            right: housing.left + i as i32 * (dw + gap) + dw,
            bottom,
        };
        if i > 0 {
            let x = cell.left - gap / 2;
            p.line(x, top, x, bottom, pal.edge, 1);
        }
        p.text_in(&ch.to_string(), cell, fonts.counter, pal.ink);
    }
}

fn punch_rect(s: f32) -> RECT {
    RECT {
        left: theme::px(PUNCH_X as f32, s),
        top: theme::px(PUNCH_Y as f32, s),
        right: theme::px((PUNCH_X + PUNCH_W) as f32, s),
        bottom: theme::px((PUNCH_Y + PUNCH_H) as f32, s),
    }
}

fn draw_autostart(p: &Painter, pal: &Palette, snap: &Snapshot, fonts: &Fonts, ui: &Ui) {
    p.text(
        "START WITH WINDOWS",
        p.d(PUNCH_X as f32),
        p.d(228.0),
        fonts.label,
        pal.dim,
        theme::LABEL_TRACKING,
    );

    let r = punch_rect(p.s);
    let on = snap.autostart;

    // State is a mark: an intact strip is OFF, a punched hole is ON. Both states
    // draw their outline at the same weight, so OFF never reads as half-disabled.
    let edge = if on || ui.hover || ui.pressed {
        pal.ink
    } else {
        pal.body
    };
    p.round(r, p.d(4.0), edge, None);

    // A switch already at ink cannot answer the pointer with a colour, so the
    // slug is what moves: pressing always takes a pixel of material, and hovering
    // over an armed switch gives one back. Feedback stays a mark, not a colour.
    let give = if ui.pressed {
        -1.0
    } else if on && ui.hover {
        1.0
    } else {
        0.0
    };
    let slug = p.d(18.0 + give * 2.0);
    let (cx, cy) = ((r.left + r.right) / 2, (r.top + r.bottom) / 2);
    let slug_r = RECT {
        left: cx - slug / 2,
        top: cy - slug / 2,
        right: cx + slug / 2,
        bottom: cy + slug / 2,
    };
    if on {
        p.round(slug_r, p.d(2.0), pal.ink, Some(pal.ink));
    } else {
        p.round(slug_r, p.d(2.0), if ui.hover || ui.pressed { pal.ink } else { pal.body }, None);
    }

    // The focus ring only appears when the keyboard is driving.
    if ui.focus {
        let ring = p.d(4.0);
        p.frame(
            RECT {
                left: r.left - ring,
                top: r.top - ring,
                right: r.right + ring,
                bottom: r.bottom + ring,
            },
            pal.dim,
        );
    }

    // The operator's tick: they armed this one.
    let wx = r.right + p.d(14.0);
    if on {
        grease(
            p,
            wx,
            cy + p.d(9.0),
            wx + p.d(5.0),
            cy - p.d(2.0),
            pal.accent,
        );
    }
    p.text_in(
        if on { "ON" } else { "OFF" },
        RECT {
            left: wx + p.d(11.0),
            top: r.top,
            right: wx + p.d(70.0),
            bottom: r.bottom,
        },
        fonts.mono,
        if on { pal.ink } else { pal.body },
    );
}

fn draw_sheet(p: &Painter, pal: &Palette, snap: &Snapshot, fonts: &Fonts, ui: &Ui) {
    let left = p.d(PAD as f32);
    let right = p.d((W - PAD) as f32);

    p.text("Ternitor", left, p.d(104.0), fonts.title, pal.ink, 0.0);
    for (i, line) in [
        "Hides the blank console windows Windows opens for processes that",
        "have no console of their own \u{2014} hidden the instant they appear,",
        "so nothing ever flashes on screen.",
    ]
    .iter()
    .enumerate()
    {
        // The measure is held against the counter housing, not by luck of font
        // metrics: if the face ever substitutes for a wider one, the line
        // ellipsizes rather than growing into the right column.
        let shown = p.fit(line, p.d(430.0), fonts.body, 0.0);
        p.text(&shown, left, p.d(142.0 + 20.0 * i as f32), fonts.body, pal.body, 0.0);
    }

    p.hrule(p.d(212.0), pal.rule);

    p.text("LAST CUT", left, p.d(228.0), fonts.label, pal.dim, theme::LABEL_TRACKING);
    if snap.hidden == 0 {
        p.text("nothing cut yet", left, p.d(248.0), fonts.mono, pal.dim, 0.0);
    } else {
        let title = if snap.last_title.is_empty() {
            "an untitled console"
        } else {
            snap.last_title.as_str()
        };
        let shown = p.fit(title, p.d(376.0), fonts.mono, 0.0);
        p.text(&shown, left, p.d(248.0), fonts.mono, pal.ink, 0.0);
    }

    for (dx, label, value) in [
        (0.0, "STARTED", clock_at(snap.session_start)),
        (136.0, "ELAPSED", span(snap.session_start.elapsed())),
    ] {
        p.text(
            label,
            left + p.d(dx),
            p.d(288.0),
            fonts.label,
            pal.dim,
            theme::LABEL_TRACKING,
        );
        p.text(&value, left + p.d(dx), p.d(304.0), fonts.mono, pal.ink, 0.0);
    }

    draw_counter(p, pal, snap, fonts);
    draw_autostart(p, pal, snap, fonts, ui);

    // The small print is also where a refused toggle explains itself. The punch
    // cannot change state on its own, so the reason takes the space already
    // there rather than adding an element.
    let x = p.d(PUNCH_X as f32);
    match &ui.autostart_error {
        Some(why) => {
            let line = p.fit(
                &format!("Couldn't change it: {why}"),
                p.d(224.0),
                fonts.small,
                0.0,
            );
            p.text(&line, x, p.d(300.0), fonts.small, pal.ink, 0.0);
            p.text(
                &format!("Ternitor is still {}.", if snap.autostart { "ON" } else { "OFF" }),
                x,
                p.d(317.0),
                fonts.small,
                pal.body,
                0.0,
            );
        }
        None => {
            for (i, line) in [
                "Runs Ternitor when you sign in. Adds one",
                "value to the registry, under HKCU.",
            ]
            .iter()
            .enumerate()
            {
                p.text(line, x, p.d(300.0 + 17.0 * i as f32), fonts.small, pal.body, 0.0);
            }
        }
    }

    p.hrule(p.d(368.0), pal.rule);

    let info = format!(
        "v{}  \u{00B7}  {}-{}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    p.text(&info, left, p.d(386.0), fonts.mono, pal.dim, 0.0);
    p.text_right("log: ternitor.log", right, p.d(386.0), fonts.mono, pal.dim, 0.0);
}

fn span(d: Duration) -> String {
    let s = d.as_secs();
    if s >= 3600 {
        format!("{}h {:02}m", s / 3600, (s % 3600) / 60)
    } else if s >= 60 {
        format!("{}m {:02}s", s / 60, s % 60)
    } else {
        format!("{s}s")
    }
}

/// The wall-clock time `start` happened at, so the session reads as a real time
/// rather than as an offset.
fn clock_at(start: Instant) -> String {
    let st = unsafe { GetLocalTime() };
    let now = st.wHour as i64 * 3600 + st.wMinute as i64 * 60 + st.wSecond as i64;
    let s = (now - start.elapsed().as_secs() as i64).rem_euclid(86400);
    format!("{:02}:{:02}", s / 3600, (s % 3600) / 60)
}

fn paint(hwnd: HWND) {
    let snap = app::snapshot();
    let pal = &theme::PALETTE;

    with_ui(|ui| {
        let mut ps = PAINTSTRUCT::default();
        let screen = unsafe { BeginPaint(hwnd, &mut ps) };
        if screen.is_invalid() {
            return;
        }

        let mut client = RECT::default();
        let _ = unsafe { GetClientRect(hwnd, &mut client) };
        let (cw, ch) = (client.right, client.bottom);
        if cw <= 0 || ch <= 0 {
            unsafe { let _ = EndPaint(hwnd, &ps); }
            return;
        }

        if ui.fonts.is_none() {
            ui.scale = theme::scale_for(hwnd);
            ui.fonts = Some(theme::fonts(ui.scale));
        }
        if !ui.ground.as_ref().is_some_and(|b| b.fits(cw, ch)) {
            ui.ground = None;
            ui.buf = None;
        }
        if ui.ground.is_none() {
            let b = Buf::new(screen, cw, ch);
            bake_ground(
                &Painter {
                    hdc: b.dc,
                    s: ui.scale,
                },
                &pal,
            );
            ui.ground = Some(b);
        }
        if ui.buf.is_none() {
            ui.buf = Some(Buf::new(screen, cw, ch));
        }

        let fonts = ui.fonts.as_ref().expect("fonts built above");
        let ground = ui.ground.as_ref().expect("ground baked above");
        let buf = ui.buf.as_ref().expect("buffer built above");

        unsafe {
            let _ = BitBlt(buf.dc, 0, 0, cw, ch, Some(ground.dc), 0, 0, SRCCOPY);
        }
        let p = Painter {
            hdc: buf.dc,
            s: ui.scale,
        };
        draw_frames(&p, &pal, &snap, ui);
        draw_sheet(&p, &pal, &snap, fonts, ui);

        unsafe {
            let _ = BitBlt(screen, 0, 0, cw, ch, Some(buf.dc), 0, 0, SRCCOPY);
            let _ = EndPaint(hwnd, &ps);
        }
    });
}

// ------------------------------------------------------------ the window ---

fn punch_hit(hwnd: HWND, x: i32, y: i32) -> bool {
    let s = with_ui(|ui| {
        let _ = hwnd;
        ui.scale
    });
    // A little slop around the punch, because it is a small target.
    let r = punch_rect(s);
    let slop = theme::px(6.0, s);
    x >= r.left - slop && x < r.right + slop && y >= r.top - slop && y < r.bottom + slop
}

/// Flip Start with Windows. The registry is the truth, so the surface re-reads it
/// on the next paint and shows the refusal if the write did not take.
fn toggle() {
    let hwnd = with_ui(|ui| ui.hwnd);
    let on = !app::snapshot().autostart;
    let err = app::set_autostart(on).err();
    with_ui(|ui| ui.autostart_error = err);
    redraw(hwnd);
}

/// Keep asking for repaints while a cut is being drawn, and stop the moment it is
/// done so that an idle Ternitor costs nothing.
fn tick(hwnd: HWND) {
    let finished = with_ui(|ui| match ui.pulse {
        Some(t0) if t0.elapsed().as_millis() as u64 >= CUT_MS + 40 => {
            ui.pulse = None;
            true
        }
        None => true,
        _ => false,
    });
    if finished && with_ui(|ui| std::mem::replace(&mut ui.ticking, false)) {
        unsafe { let _ = KillTimer(Some(hwnd), TIMER_ID); }
    }
    unsafe { let _ = InvalidateRect(Some(hwnd), None, false); }
}

fn redraw(hwnd: HWND) {
    unsafe { let _ = InvalidateRect(Some(hwnd), None, false); }
}

unsafe extern "system" fn sheet_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            paint(hwnd);
            LRESULT(0)
        }
        // Everything is painted; there is no background to erase.
        WM_ERASEBKGND => LRESULT(1),
        WM_TIMER => {
            tick(hwnd);
            LRESULT(0)
        }
        // One screen, one size. Pinning the track sizes to the size we were built
        // for means the layout can never be asked to paint a client it has no
        // metrics for -- not by the resize border, not by snap, not by Win+Up.
        WM_GETMINMAXINFO => {
            let r = DefWindowProcW(hwnd, msg, wparam, lparam);
            let info = lparam.0 as *mut MINMAXINFO;
            if !info.is_null() {
                let outer = outer_size(theme::scale_for(hwnd));
                let i = unsafe { &mut *info };
                i.ptMinTrackSize = POINT { x: outer.0, y: outer.1 };
                i.ptMaxTrackSize = POINT { x: outer.0, y: outer.1 };
            }
            r
        }
        WM_MOUSEMOVE => {
            let over = punch_hit(hwnd, word(lparam.0, 0), word(lparam.0, 16));
            if with_ui(|ui| std::mem::replace(&mut ui.hover, over) != over) {
                redraw(hwnd);
            }
            let mut tme = TRACKMOUSEEVENT {
                cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                dwFlags: TME_LEAVE,
                hwndTrack: hwnd,
                dwHoverTime: 0,
            };
            let _ = TrackMouseEvent(&mut tme);
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            if with_ui(|ui| std::mem::replace(&mut ui.hover, false)) {
                redraw(hwnd);
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            if punch_hit(hwnd, word(lparam.0, 0), word(lparam.0, 16)) {
                with_ui(|ui| ui.pressed = true);
                redraw(hwnd);
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let was = with_ui(|ui| std::mem::replace(&mut ui.pressed, false));
            if was && punch_hit(hwnd, word(lparam.0, 0), word(lparam.0, 16)) {
                with_ui(|ui| ui.focus = true);
                toggle();
            }
            redraw(hwnd);
            LRESULT(0)
        }
        WM_SETFOCUS => {
            with_ui(|ui| ui.focus = true);
            redraw(hwnd);
            LRESULT(0)
        }
        // WA_INACTIVE: the keyboard is no longer driving, so the ring goes away.
        WM_ACTIVATE => {
            if uword(wparam.0, 0) == 0 && with_ui(|ui| std::mem::replace(&mut ui.focus, false)) {
                redraw(hwnd);
            }
            LRESULT(0)
        }
        WM_KEYDOWN => {
            match wparam.0 as u16 {
                k if k == VK_SPACE.0 || k == VK_RETURN.0 => toggle(),
                k if k == VK_ESCAPE.0 => {
                    let _ = ShowWindow(hwnd, SW_HIDE);
                }
                _ => return DefWindowProcW(hwnd, msg, wparam, lparam),
            }
            LRESULT(0)
        }
        WM_SETCURSOR => {
            let mut pt = POINT::default();
            let _ = GetCursorPos(&mut pt);
            let _ = ScreenToClient(hwnd, &mut pt);
            if punch_hit(hwnd, pt.x, pt.y) {
                if let Ok(c) = LoadCursorW(None, IDC_HAND) {
                    SetCursor(Some(c));
                }
                return LRESULT(1);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_DPICHANGED => {
            // lparam points at the rect Windows suggests; take it, and rebuild
            // every metric from the new scale.
            let suggested = lparam.0 as *const RECT;
            if !suggested.is_null() {
                let r = *suggested;
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    r.left,
                    r.top,
                    r.right - r.left,
                    r.bottom - r.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
            with_ui(|ui| ui.drop_cache());
            redraw(hwnd);
            LRESULT(0)
        }
        WM_CLOSE => {
            // Closing means back to the tray, not quitting: the app is still
            // working, and quitting is the tray menu's job.
            let _ = ShowWindow(hwnd, SW_HIDE);
            LRESULT(0)
        }
        WM_DESTROY => {
            with_ui(|ui| ui.drop_cache());
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// The `bit`-th 16-bit word of a packed message parameter, sign-extended. LPARAM
/// arrives as `isize` and WPARAM as `usize`, hence the two.
fn word(v: isize, bit: u32) -> i32 {
    ((v >> bit) as u16) as i16 as i32
}

fn uword(v: usize, bit: u32) -> i32 {
    ((v >> bit) as u16) as i16 as i32
}

/// The window's chrome for the fixed client, so every path that needs the outer
/// size agrees on it: creation, and the clamp that stops the window resizing.
fn outer_size(scale: f32) -> (i32, i32) {
    let style = WINDOW_STYLE(WS_CAPTION.0 | WS_SYSMENU.0 | WS_MINIMIZEBOX.0);
    let mut r = RECT {
        left: 0,
        top: 0,
        right: theme::px(W as f32, scale),
        bottom: theme::px(H as f32, scale),
    };
    let _ = unsafe { AdjustWindowRectEx(&mut r, style, false, WINDOW_EX_STYLE(0)) };
    (r.right - r.left, r.bottom - r.top)
}

fn create() -> Result<HWND, String> {
    let class = wide("TernitorBench");
    let style = WINDOW_STYLE(WS_CAPTION.0 | WS_SYSMENU.0 | WS_MINIMIZEBOX.0);
    let cursor = unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default();

    let wc = WNDCLASSW {
        lpfnWndProc: Some(sheet_proc),
        hInstance: instance(),
        hCursor: cursor,
        lpszClassName: pcw(&class),
        ..Default::default()
    };
    unsafe { RegisterClassW(&wc) };

    let (ow, oh) = outer_size(theme::scale_for_system());

    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            pcw(&class),
            pcw(&wide("Ternitor")),
            style,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            ow,
            oh,
            None,
            None,
            Some(instance()),
            None,
        )
    }
    .map_err(|e| format!("CreateWindowExW failed: {e}"))?;

    apply_caption(hwnd);
    Ok(hwnd)
}

/// The caption is part of the bench: one black plate, not a black plate sitting
/// inside a light Windows frame. Best effort -- older Windows builds ignore these
/// attributes and keep the stock caption.
fn apply_caption(hwnd: HWND) {
    let pal = &theme::PALETTE;
    let dark = 1i32;
    let (caption, border, text) = (pal.ground as i32, pal.ground as i32, pal.ink as i32);
    unsafe {
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, ptr(&dark), 4);
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_CAPTION_COLOR, ptr(&caption), 4);
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_BORDER_COLOR, ptr(&border), 4);
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_TEXT_COLOR, ptr(&text), 4);
    }
}

fn ptr(v: &i32) -> *const std::ffi::c_void {
    v as *const i32 as *const std::ffi::c_void
}

/// Show the settings sheet, creating it on first use.
pub fn open() {
    if with_ui(|ui| ui.hwnd).is_invalid() {
        match create() {
            Ok(h) => {
                with_ui(|ui| ui.hwnd = h);
                crate::log::write("settings window opened");
            }
            Err(e) => {
                crate::log::write(&format!("settings window failed: {e}"));
                return;
            }
        }
    }
    let hwnd = with_ui(|ui| ui.hwnd);
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
        let _ = SetForegroundWindow(hwnd);
    }
}

/// State changed without a window being hidden -- a setting, or the pause from
/// the tray. Repaint, but do not spend the cut mark: that gesture means one
/// thing, and it is not "a checkbox moved".
pub fn refresh() {
    with_ui(|ui| {
        if !ui.hwnd.is_invalid() {
            unsafe { let _ = InvalidateRect(Some(ui.hwnd), None, false); }
        }
    });
}

/// Something changed. Start the cut mark, but only if anyone is watching.
pub fn notify_activity() {
    with_ui(|ui| {
        if ui.hwnd.is_invalid() || !unsafe { IsWindowVisible(ui.hwnd).as_bool() } {
            return;
        }
        if theme::animations_enabled() {
            ui.pulse = Some(Instant::now());
            if !ui.ticking {
                ui.ticking = true;
                unsafe { SetTimer(Some(ui.hwnd), TIMER_ID, FRAME_MS, None) };
            }
        }
        unsafe { let _ = InvalidateRect(Some(ui.hwnd), None, false); }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A memory DC with the real fonts on it, so the measurements below are the
    /// ones the surface actually draws with.
    fn bench<R>(f: impl FnOnce(&Painter, &Fonts) -> R) -> R {
        let scale = 1.0;
        let dc = unsafe { CreateCompatibleDC(None) };
        let fonts = theme::fonts(scale);
        let r = f(&Painter { hdc: dc, s: scale }, &fonts);
        fonts.destroy();
        unsafe { let _ = DeleteDC(dc); }
        r
    }

    /// The counter's column, `PUNCH_X` to the right margin. The paused indicator
    /// replaces the counter's label with two words instead of one, and must stay
    /// inside the same column.
    #[test]
    fn the_paused_label_fits_the_counter_column() {
        let column = theme::px((W - PAD - PUNCH_X) as f32, 1.0);
        let w = bench(|p, f| {
            p.measure("PAUSED", f.label, theme::LABEL_TRACKING)
                + theme::px(12.0, 1.0)
                + p.measure("NOT COUNTING", f.label, theme::LABEL_TRACKING)
        });
        assert!(w <= column, "paused label is {w}px in a {column}px column");
    }

    /// The paragraph is held to a fixed measure, and that measure has to stop
    /// short of the widest counter a session can realistically reach.
    #[test]
    fn the_paragraph_measure_stops_before_the_counter() {
        let para_right = PAD + 430;
        let housing_left = W - PAD - (3 * DIGIT_W + 2 * DIGIT_GAP);
        assert!(
            para_right < housing_left,
            "paragraph reaches {para_right}, a three-digit counter starts at {housing_left}"
        );
    }
}

/// Tear the sheet down. Only called on the way out.
pub fn close() {
    let hwnd = with_ui(|ui| ui.hwnd);
    if !hwnd.is_invalid() {
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
    }
}
