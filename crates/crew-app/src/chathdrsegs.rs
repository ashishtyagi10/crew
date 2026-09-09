//! The agent smith header's status segments: WHAT the right-aligned status
//! says — spinner, active agent, compact chip, busy hint, connection dot —
//! and the colour each piece wears. `chathdr` lays them out; this is split
//! from it along that line for the 200-line cap.
use crate::glyphs::{pick, spinner, Glyph};
use crate::shimmer::Color;

/// One status segment: styled chars, laid left to right. Per-char colour is
/// what lets the `thinking` word carry its shimmer (`shimmer::cells`).
pub(crate) type Seg = Vec<(char, Color)>;

pub(crate) fn seg(s: &str, fg: Color) -> Seg {
    s.chars().map(|c| (c, fg)).collect()
}

/// The muted hint appended to the status while the pane is busy — Esc
/// cancels the running turn instead of closing the pane (see the
/// esc-interrupt design doc). Its own segment, inserted right before the
/// connection dot, so [`header_cells`] can drop it first — before touching
/// anything else — when the pane is too narrow for the full status.
const INTERRUPT_HINT: &str = "\u{00b7} esc interrupts";

/// The muted chip shown while the transcript is in compact view (Ctrl+O —
/// see `ChatPane::compact_view`). Same segment family as [`INTERRUPT_HINT`]
/// (an optional, droppable `· label` suffix), but less essential than the
/// busy hint, so [`header_cells`] drops it first when the pane is too narrow
/// for the full status — before touching the esc-interrupts hint.
const COMPACT_CHIP: &str = "\u{00b7} compact";

/// The right-aligned status segments, in left-to-right order.
/// While an agent is active the spinner names it and counts the elapsed
/// seconds (`| coder · 12s`, in the colour the caller chose — the roster
/// colour, lit by `shimmer::pulse_color` while its tokens flow); otherwise a
/// `thinking` spinner appears while a send is unanswered, the word wearing
/// the shimmer (muted, with the accent sweeping across it). The trailing
/// connection dot keeps the tighter single-space gap it always had, and
/// breathes (`shimmer::breath_color`) while the pane is connected and idle.
/// Session stats (model, context, tokens) live in the below-input summary
/// footer (`chatsummary`) — the header is identity and liveness only.
/// `hint`, when the pane is busy, adds the muted "esc interrupts" segment
/// just before the dot — callers drop it (`hint: false`) to reclaim width on
/// narrow panes. `compact`, when the transcript is in compact view, adds the
/// muted "compact" chip after the spinner (dropped first of the two —
/// see [`COMPACT_CHIP`] — via `compact: false`).
/// `tools` (calls in flight, see `chattool`) prefixes the hint with
/// `· 2 tools running` — the thing Esc would actually interrupt.
pub(crate) fn status_segments(
    connected: bool,
    awaiting: bool,
    active: Option<(&str, u64, Color)>,
    compact: bool,
    hint: bool,
    tools: usize,
    now_ms: u64,
) -> Vec<Seg> {
    let t = crew_theme::theme();
    let level = crate::motion::level();
    let accent = crate::palette::accent();
    let mut segs = Vec::new();
    // The spinner: ASCII strokes, or pie slices on a Nerd Font (`glyphs`).
    let spin = spinner(now_ms);
    if let Some((label, secs, color)) = active {
        segs.push(seg(&format!("{spin} {label} \u{00b7} {secs}s"), color));
    } else if awaiting {
        let mut s = seg(&format!("{spin} "), accent);
        let ms = crate::shimmer::SHIMMER_MS;
        s.extend(crate::shimmer::cells(
            "thinking",
            now_ms,
            t.text_muted,
            accent,
            t.page_bg,
            ms,
            level,
        ));
        segs.push(s);
    }

    // Compact-view chip, width-permitting — appended after the spinner,
    // ahead of the busy hint (see `header_cells`: it's the first of the two
    // dropped on a narrow pane).
    if compact {
        segs.push(seg(COMPACT_CHIP, t.text_muted));
    }

    // Busy hint, width-permitting (see `header_cells`) — appended after the
    // counters (and the compact chip, if shown), before the connection dot.
    if awaiting && hint {
        let running = match tools {
            0 => String::new(),
            1 => "\u{00b7} 1 tool running ".to_string(),
            n => format!("\u{00b7} {n} tools running "),
        };
        segs.push(seg(&format!("{running}{INTERRUPT_HINT}"), t.text_muted));
    }

    // ● connected (breathing while idle; steady while busy — the spinner is
    // the liveness then), ○ connecting.
    segs.push(if connected && awaiting {
        seg(pick(Glyph::DotOn), t.activity)
    } else if connected {
        let fg = crate::shimmer::breath_color(now_ms, level, t.dim, t.activity, t.page_bg);
        seg(pick(Glyph::DotOn), fg)
    } else {
        seg(pick(Glyph::DotOff), t.dim)
    });
    segs
}
