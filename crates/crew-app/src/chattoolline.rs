//! How one tool line draws (see [`crate::chattool`]): the pending form with
//! its spinner and counting clock, the done form with its mark and timing in
//! the added/removed inks, and the opened result text on the code field.
//! Pure — a line, a clock, a width and the icon-set switch in; cells out.
use crate::chatbody::{plain, CardCell, CardLine};
use crate::chattool::ToolLine;
use crate::chattoolkind::LineKind;
use crate::chatwidth::{char_w, clip_w, str_w};
use crate::glyphs::{fallback, nerd, Glyph};

/// The most rows an opened result text shows.
pub(crate) const TEXT_ROWS: usize = 12;
/// Characters of the call's subject kept on the line.
const ARGS_CHARS: usize = 60;
/// Argument names that ARE the call (the broker's `toolline` rule).
const HEADLINE: &[&str] = &["cmd", "command", "path", "file", "url", "query", "pattern"];

/// The argument worth showing: the headline field of a JSON object (or its
/// only string field), else the raw arguments — flattened and clipped.
pub(crate) fn subject(args: &str) -> String {
    let flat = |s: &str| -> String {
        let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
        crate::chatwidth::clip_w(&s, ARGS_CHARS)
    };
    let obj = serde_json::from_str::<serde_json::Value>(args)
        .ok()
        .and_then(|v| v.as_object().cloned());
    if let Some(obj) = obj {
        let field = |k: &str| obj.get(k).and_then(|v| v.as_str()).map(flat);
        if let Some(s) = HEADLINE.iter().find_map(|k| field(k)) {
            return s;
        }
        if let [only] = obj.values().collect::<Vec<_>>()[..] {
            if let Some(s) = only.as_str() {
                return flat(s);
            }
        }
        if obj.is_empty() {
            return String::new();
        }
    }
    flat(args)
}

/// `120 ms` under a second, `3.2 s` under a minute, `1m 04s` past one.
pub(crate) fn fmt_ms(ms: u64) -> String {
    match ms {
        0..=999 => format!("{ms} ms"),
        1000..=59_999 => format!("{:.1} s", ms as f64 / 1000.0),
        _ => format!("{}m {:02}s", ms / 60_000, (ms / 1000) % 60),
    }
}

/// `g` on the icon set (`on`) or its plain fallback.
pub(crate) fn glyph(g: Glyph, on: bool) -> &'static str {
    if on {
        nerd(g)
    } else {
        fallback(g)
    }
}

/// The mark, its colour, the subject's colour and the timing text for a line
/// at `now_ms`. Pending: the spinner frame the clock picks (a static `…` at
/// motion Off), whole seconds elapsed once a second has passed — so the
/// clock counts up live but never changes more than once a second.
fn parts(line: &ToolLine, now_ms: u64, on: bool) -> (&'static str, Color, Color, Option<String>) {
    use crate::md::syntax::Token;
    let th = crew_theme::theme();
    // A load: its kind's mark in the accent, and no clock — nothing ran.
    if line.kind != LineKind::Tool {
        return (
            glyph(line.kind.glyph(), on),
            crate::palette::accent(),
            th.ink,
            None,
        );
    }
    match &line.done {
        None => {
            let mark = match crate::motion::level() {
                crate::motion::MotionLevel::Off => "\u{2026}",
                _ => crate::glyphs::spinner_on(now_ms, on),
            };
            let secs = now_ms.saturating_sub(line.started_ms) / 1000;
            let tail = (secs > 0).then(|| format!("{secs}s"));
            (mark, crate::palette::accent(), th.text_muted, tail)
        }
        Some(d) => {
            let (g, tok) = match d.ok {
                true => (Glyph::Pass, Token::Added),
                false => (Glyph::Fail, Token::Removed),
            };
            let fg = crate::chatink::token_fg(tok);
            (glyph(g, on), fg, th.ink, Some(fmt_ms(d.ms)))
        }
    }
}

type Color = crate::chatbody::Color;

/// One line as the card draws it, `cols` wide: `  ⟳ label subject · 3s`
/// while pending, `  ✓ label subject · 120 ms` once done (`✗` on failure),
/// then the result's first line muted in whatever room is left. The
/// subject is clipped so the timing always fits. A load reads
/// `  ✦ skill rust-testing · applied · tests first`, its detail in the
/// timing's place and held to half the row so the name always shows. `on`
/// = the Nerd Font icon set is active.
pub(crate) fn render(line: &ToolLine, now_ms: u64, cols: usize, on: bool) -> CardLine {
    let muted = crew_theme::theme().text_muted;
    let (mark, mark_fg, fg, tail) = parts(line, now_ms, on);
    let tail = match (&line.done, line.kind) {
        (Some(d), k) if k != LineKind::Tool => Some(clip_w(&d.text, cols / 2)),
        _ => tail,
    };
    let tail = tail.map_or(String::new(), |t| format!(" \u{00b7} {t}"));
    let subject = match line.args_short.is_empty() {
        true => line.label.clone(),
        false => format!("{} {}", line.label, line.args_short),
    };
    let head_w = 3 + str_w(mark);
    let subject = clip_w(&subject, cols.saturating_sub(head_w + str_w(&tail)));
    let mut out: CardLine = "  ".chars().map(|c| plain(c, muted, false)).collect();
    out.extend(mark.chars().map(|c| plain(c, mark_fg, false)));
    out.push(plain(' ', muted, false));
    out.extend(subject.chars().map(|c| plain(c, fg, false)));
    out.extend(tail.chars().map(|c| plain(c, muted, false)));
    let used: usize = out.iter().map(|c| char_w(c.c)).sum();
    let first = line
        .done
        .as_ref()
        .filter(|_| line.kind == LineKind::Tool)
        .and_then(|d| d.text.lines().find(|l| !l.trim().is_empty()));
    if let Some(first) = first.filter(|_| cols > used + 4) {
        let preview = format!("  {}", clip_w(first.trim(), cols - used - 2));
        out.extend(preview.chars().map(|c| plain(c, muted, false)));
    }
    out
}

/// The rows opened under a clicked line — the arguments, `→ result`, up to
/// [`TEXT_ROWS`] of the result (`chattoolargs::opened_rows` decides them) —
/// on the code field, muted, each clipped to the width. Empty unless the
/// line was clicked open.
pub(crate) fn text_rows(line: &ToolLine, cols: usize) -> Vec<CardLine> {
    if !line.show_text {
        return Vec::new();
    }
    let (bg, fg) = (crate::chatink::code_bg(), crew_theme::theme().text_muted);
    let cell = |c: char| CardCell {
        bg: Some(bg),
        ..plain(c, fg, false)
    };
    crate::chattoolargs::opened_rows(line)
        .into_iter()
        .map(|s| {
            let body = format!("    {}", clip_w(&s, cols.saturating_sub(4)));
            let pad = cols.saturating_sub(str_w(&body));
            body.chars()
                .chain(std::iter::repeat_n(' ', pad))
                .map(cell)
                .collect()
        })
        .collect()
}

#[cfg(test)]
#[path = "chattoolline_tests.rs"]
mod tests;
