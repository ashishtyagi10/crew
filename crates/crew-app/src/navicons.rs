//! The activity bar's icons, drawn rather than typed.
//!
//! As glyphs they were one cell each: a Codicon squeezed into a monospace cell on a Nerd Font,
//! and a plain mark (`▦ ◷ ↯ ❐ ⚙`) on any other face — which is most faces, the user's own
//! included. Neither was the size of the icons the bar was modelled on. Drawn on the paint layer
//! they are the same everywhere, any font, and as big as the bar is wide: outline shapes in the
//! Codicon manner, a hairline stroke with round caps, two and a half columns across.
//!
//! Each is a signed distance ([`crate::plot::sdf`]) — so a stroke thinner than a canvas pixel
//! still lands as a grey edge instead of vanishing — built in a box of half-size `u` around its
//! centre, and filled in the colour the glyph used to wear.
use crew_render::Paint;

use crate::navbar::BarHit;
use crate::navsection::NavSection;
use crate::plot::sdf;

/// How wide an icon is, in columns.
const SIZE: f32 = 2.6;
/// Half the stroke, as a share of the icon's half-size.
const STROKE: f32 = 0.07;

/// Where `hit`'s icon is centred in a bar `rows` tall, in columns and ROWS — the bar's middle
/// column, the middle of the icon's row. `None` when the bar has no room for it.
pub(crate) fn centre(hit: BarHit, rows: u16) -> Option<(f32, f32)> {
    let row = crate::navbar::row_of(hit, rows)?;
    Some((
        f32::from(crate::navbar::BAR_COLS) * 0.5,
        f32::from(row) + 0.5,
    ))
}

/// The bar's icons as paint, in its own cell coordinates. Colours as the glyphs had them: the
/// open section's in the accent, the hovered one's in ink, the rest muted.
pub(crate) fn bar_paint(
    open: Option<NavSection>,
    hover: Option<BarHit>,
    rows: u16,
    aspect: f32,
) -> Vec<Paint> {
    // Seven static shapes: rasterised again only when something they depend on changed — what
    // is open or hovered, the bar's height, the cell's shape and size, the theme or accent.
    let key = (
        open,
        hover,
        rows,
        aspect.to_bits(),
        crate::plot::device::sub(),
        crew_theme::current_id(),
        crate::palette::accent(),
    );
    LAST.with(|last| {
        let mut last = last.borrow_mut();
        if let Some((k, paint)) = last.as_ref().filter(|(k, _)| *k == key) {
            let _ = k;
            return paint.clone();
        }
        let paint = draw(open, hover, rows, aspect);
        *last = Some((key, paint.clone()));
        paint
    })
}

type Key = (
    Option<NavSection>,
    Option<BarHit>,
    u16,
    u32,
    usize,
    crew_theme::ThemeId,
    (u8, u8, u8),
);

thread_local! {
    static LAST: std::cell::RefCell<Option<(Key, Vec<Paint>)>> = const { std::cell::RefCell::new(None) };
}

fn draw(open: Option<NavSection>, hover: Option<BarHit>, rows: u16, aspect: f32) -> Vec<Paint> {
    let t = crew_theme::theme();
    let accent = crate::accentink::accent_ink();
    let mut c = crate::plot::Canvas::new(crate::navbar::BAR_COLS, rows, aspect);
    let hits = NavSection::ALL.into_iter().map(BarHit::Section);
    for hit in hits.chain([BarHit::Settings]) {
        let Some((x, row)) = centre(hit, rows) else {
            continue;
        };
        let fg = match (hit, hover == Some(hit)) {
            (BarHit::Section(s), _) if open == Some(s) => accent,
            (_, true) => t.ink,
            _ => t.text_muted,
        };
        // Canvas units are square: a row is `aspect` units tall.
        let at = (x, row * aspect);
        let u = SIZE * 0.5;
        let bbox = (at.0 - u - 0.3, at.1 - u - 0.3, 2.0 * u + 0.6, 2.0 * u + 0.6);
        c.fill_sdf(bbox, fg, 1.0, |px, py| shape(hit, (px, py), at, u));
    }
    c.paint()
}

/// The icon for `hit` as a distance at `p`, centred on `at` with half-size `u`.
pub(crate) fn shape(hit: BarHit, p: (f32, f32), at: (f32, f32), u: f32) -> f32 {
    // Local coordinates: the icon's box is -1..1 on both axes; distances come back in units.
    let l = |x: f32, y: f32| (at.0 + x * u, at.1 + y * u);
    let w = STROKE * u;
    let outline = |d: f32| d.abs() - w;
    let line = |a: (f32, f32), b: (f32, f32)| sdf::capsule(p, l(a.0, a.1), l(b.0, b.1), w);
    let rbox = |x: f32, y: f32, bw: f32, bh: f32| {
        let (x0, y0) = l(x, y);
        sdf::round_box(p, x0, y0, bw * u, bh * u, 0.16 * u)
    };
    match hit {
        // A window split into panes: crew's grid, as the layout icons of an editor draw one.
        BarHit::Section(NavSection::Panes) => {
            let frame = outline(rbox(-0.9, -0.8, 1.8, 1.6));
            let split = line((0.0, -0.8), (0.0, 0.8));
            let half = line((0.0, 0.0), (0.9, 0.0));
            frame.min(split).min(half)
        }
        // A gauge: an open arc, a needle, a hub.
        BarHit::Section(NavSection::System) => {
            let hub = l(0.0, 0.25);
            let arc = sdf::arc(p, hub, 0.85 * u, w, -2.2, 2.2);
            let tip = sdf::polar(hub, 0.62 * u, 0.8);
            let needle = sdf::capsule(p, hub, tip, w * 1.2);
            arc.min(needle).min(sdf::disc(p, hub, 0.13 * u))
        }
        // A pulse: a flat line with one beat in it.
        BarHit::Section(NavSection::Activity) => {
            let pts = [
                (-1.0, 0.1),
                (-0.45, 0.1),
                (-0.2, -0.75),
                (0.2, 0.85),
                (0.45, 0.1),
                (1.0, 0.1),
            ];
            pts.windows(2)
                .map(|s| line(s[0], s[1]))
                .fold(f32::MAX, f32::min)
        }
        // Two pages, the front one with its corner folded — the files icon VS Code's explorer
        // wears.
        BarHit::Section(NavSection::Explorer) => {
            let back = rbox(-0.85, -0.95, 1.25, 1.5);
            let (fx, fy, fw, fh) = (-0.4, -0.55, 1.25, 1.5);
            let page = rbox(fx, fy, fw, fh);
            // The corner beyond the fold is cut away; the cut's own edge is the fold.
            let a = l(fx + fw - 0.45, fy);
            let cut = ((p.0 - a.0) - (p.1 - a.1)) * std::f32::consts::FRAC_1_SQRT_2;
            let front = page.max(cut);
            outline(back).max(-front).min(outline(front))
        }
        BarHit::Section(NavSection::Inbox) => crate::naviconsmore::inbox(p, at, u, w),
        BarHit::Section(NavSection::Far) => crate::naviconsmore::far(p, at, u, w),
        // A gear: a ring with eight teeth and a hole.
        BarHit::Settings => {
            let c = l(0.0, 0.0);
            // The body and its teeth as ONE solid, then outlined — a gear, not a ring with
            // spokes, which read as a sun.
            let body = sdf::disc(p, c, 0.62 * u);
            let teeth = (0..8)
                .map(|k| {
                    let a = k as f32 * std::f32::consts::FRAC_PI_4;
                    let (a0, a1) = (sdf::polar(c, 0.6 * u, a), sdf::polar(c, 0.82 * u, a));
                    sdf::capsule(p, a0, a1, 0.15 * u)
                })
                .fold(f32::MAX, f32::min);
            outline(body.min(teeth)).min(sdf::ring(p, c, 0.24 * u, w))
        }
    }
}

#[cfg(test)]
#[path = "navicons_tests.rs"]
mod tests;
