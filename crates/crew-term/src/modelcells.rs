//! Rendering the grid to `RenderCell`s, plus the program-painted-background
//! heuristics that decide when a highlight is dropped to the canvas. Split
//! from `model.rs` (child module — parent-private access preserved).
use super::*;

/// Background painted over selected cells.
pub(super) const SELECTION_BG: (u8, u8, u8) = (54, 84, 130);

/// A desaturated (grey) background at either extreme — the kind agent CLIs
/// paint behind the line you just sent, tuned to whichever theme they detected
/// at startup: dark grey on a dark guess (`ESC[48;2;55;55;55m`), light grey on
/// a light one (`ESC[48;2;230;230;230m`). After a live theme switch the
/// opposite-theme variant lands as a glaring block (white word-boxes on the
/// dark canvas). The `≤24` channel spread keeps the match grey-only, and the
/// dark `≤96` / light `≥160` bounds leave mid-greys and every saturated
/// background that carries meaning (diff red/green, error rows) untouched.
fn is_echo_grey((r, g, b): (u8, u8, u8)) -> bool {
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    (mx <= 96 || mn >= 160) && mx - mn <= 24
}

/// Whether a program-painted background should be dropped to the canvas.
///
/// In a dark theme, agent CLIs paint "highlight" backgrounds tuned to
/// whichever theme they detected at startup — dark grey, light grey, or
/// (after a live switch) the opposite-theme variant — and any of those reads
/// as an ugly box on the flat dark canvas regardless of how close to the
/// extremes it sits. So in dark mode we drop any low-saturation (`≤24`
/// channel spread) background, MID-grey included, plus any background bright
/// enough to itself read as a light "highlight" box (luminance `> 0.6`) even
/// if it happens to carry a little saturation. Saturated *dark* colours
/// (spread `> 24` and luminance `≤ 0.6` — diff red/green, error rows) survive
/// untouched.
///
/// In a light theme the existing (dark/light-extreme-only) echo-grey
/// behaviour is unchanged.
pub(super) fn should_drop_bg((r, g, b): (u8, u8, u8), dark: bool) -> bool {
    if dark {
        let mx = r.max(g).max(b);
        let mn = r.min(g).min(b);
        (mx - mn <= 24) || crate::contrast::luminance((r, g, b)) > 0.6
    } else {
        is_echo_grey((r, g, b))
    }
}

impl TermCore {
    pub(crate) fn cells(&self, focused: bool) -> Vec<RenderCell> {
        let content = self.term.renderable_content();
        let palette = content.colors;
        // When scrolled into history, viewport lines are negative; add the display
        // offset to map each line back to a 0-based viewport row.
        let off = content.display_offset as i32;
        let cursor = content.cursor;
        let selection = content.selection;
        let dark = crate::contrast::luminance(default_bg()) < 0.5;
        let mut out: Vec<RenderCell> = content
            .display_iter
            .filter(|ind| ind.c != ' ' && ind.c != '\0' && ind.point.line.0 + off >= 0)
            .map(|ind| {
                let bold = ind.flags.contains(Flags::BOLD);
                let italic = ind.flags.contains(Flags::ITALIC);
                let fg = resolve_color(ind.fg, palette, default_fg());
                let mut bg = resolve_color(ind.bg, palette, default_bg());
                // Reverse-video (SGR 7) is intentionally NOT honoured: programs
                // (e.g. agent CLIs) use it to "highlight" the line you just sent,
                // which renders as a hard-to-read block. Dropping the fg/bg swap
                // shows that text plainly instead.
                // Agent CLIs (Claude/codex) also paint the just-sent line with a
                // real near-grey background tuned to the theme they detected at
                // startup — dark grey on dark, light grey on light — which reads
                // as a muddy (or glaring) block on the actual canvas. Drop those
                // echo greys so the text shows plainly, while keeping saturated
                // backgrounds that carry meaning (diffs, errors). In a dark
                // theme this also flattens MID-grey and any light "highlight"
                // background (see `should_drop_bg`), since the flat-canvas
                // vision is stricter there than the light-theme extremes-only
                // check.
                if should_drop_bg(bg, dark) {
                    bg = default_bg();
                }
                // Selected cells take the selection background, drawn over any
                // program colours (the copied text comes from the engine).
                if selection.is_some_and(|r| r.contains(ind.point)) {
                    bg = SELECTION_BG;
                }
                // Legibility floor: a program that sampled the background once
                // (or guessed wrong) keeps painting for the other theme after a
                // live switch — nudge any too-close fg until it reads.
                let fg = crate::contrast::ensure_min_contrast(fg, bg);
                RenderCell {
                    col: ind.point.column.0 as u16,
                    row: (ind.point.line.0 + off) as u16,
                    c: ind.c,
                    fg,
                    bg,
                    bold,
                    italic,
                }
            })
            .collect();
        crate::cursor::apply(&mut out, &cursor, off, focused);
        out
    }
}
