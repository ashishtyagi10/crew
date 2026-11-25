//! Rung → `Vec<CardLine>`. Every format lands in the same representation the
//! chat cards use, so `render` is one mapper and each rung is tested as data.
//! Syntax colouring lives in `codepaint` and the opaque/metadata card in
//! `metacard` — both split out to keep this file under the length budget.
pub(crate) use super::linepaint::*;
use crate::chatbody::CardLine;
use crate::viewpane::detect::Format;
use crate::viewpane::load::{Loaded, MAX_VIEW_BYTES};
use crate::viewpane::metacard::{fmt_size, opaque_card};
use crate::viewpane::outline::Mark;
use crate::viewpane::rendercap::{cap_render_lines, MAX_RENDER_LINES};

pub(super) fn banner(msg: &str, cols: usize) -> CardLine {
    let t = crew_theme::theme();
    let mut s = crate::chatwidth::clip_w(msg, cols.max(1));
    while s.chars().count() < cols {
        s.push(' ');
    }
    row(&s, t.text_muted, false)
}

fn mb(bytes: u64) -> u64 {
    bytes / (1024 * 1024)
}

#[cfg(test)]
pub(crate) use super::linestate::for_state;
pub(crate) use super::linestate::for_state_revealed;
#[cfg(test)]
use crate::viewpane::LoadState;

/// Landmarks moved down past the banners drawn above the body.
fn shifted(marks: Vec<Mark>, above: usize) -> Vec<Mark> {
    marks
        .into_iter()
        .map(|m| Mark {
            row: m.row + above,
            ..m
        })
        .collect()
}

pub(super) fn ready_lines(
    format: Format,
    loaded: &Loaded,
    raw: bool,
    cols: usize,
    (invisibles, split): (bool, bool),
    reveal: Option<crate::md::reveal::Reveal>,
) -> (Vec<CardLine>, Vec<Mark>, Vec<crate::chatmd::Picture>) {
    let t = crew_theme::theme();
    let mut out = Vec::new();
    // Pictures a document NAMES (`![alt](src)`), in output rows. Only the
    // markdown rung reserves any; every other rung leaves this empty.
    let mut pictures: Vec<crate::chatmd::Picture> = Vec::new();
    if let Some(real) = loaded.truncated {
        out.push(banner(
            &format!(
                "showing first {} MB of {} MB — press o to open externally",
                mb(MAX_VIEW_BYTES),
                mb(real)
            ),
            cols,
        ));
    }
    // Fix 5: cap SOURCE lines before any rung renders them, not the RESULT
    // afterward — truncating the output would still pay the full render
    // cost this cap exists to avoid.
    let (text, capped_from) = cap_render_lines(&loaded.text);
    if let Some(real_lines) = capped_from {
        // `real_lines` counts lines in `loaded.text`, which is itself the 8
        // MB VIEW cap's slice whenever `loaded.truncated` is `Some` — an
        // exact count would then be naming the slice's line count as if it
        // were the file's, understating a file that runs on past the byte
        // cap. "at least" keeps the claim true either way.
        let of_lines = if loaded.truncated.is_some() {
            format!("at least {real_lines}")
        } else {
            real_lines.to_string()
        };
        out.push(banner(
            &format!(
                "showing first {MAX_RENDER_LINES} of {of_lines} lines — press o to open externally"
            ),
            cols,
        ));
    }
    // Tabs are expanded HERE, before any rung sees the text, so the syntax
    // paint, the wrap, the search and the diff pairing all agree about which
    // column a character is in. See `super::whitespace` — a tab has zero
    // display width, so an unexpanded one was not merely misaligned, it was
    // dropped, and a tab-indented file drew with no indentation at all.
    let prepared = super::whitespace::prepare(text, invisibles);
    let (text, ws) = (prepared.text.as_str(), prepared.marks.as_slice());
    let mut marks = Vec::new();
    let body = match format {
        Format::Opaque { why } => opaque_card(why, loaded.meta.as_ref(), cols),
        // The picture itself is drawn, not spelled (see `super::bitmap`) —
        // all this rung writes is the banner over it, which is also what the
        // scroll, the search and the position readout count.
        Format::Image { kind } => {
            let size = loaded.meta.map_or(String::new(), |m| {
                format!("  \u{00b7}  {}", fmt_size(m.size))
            });
            let dims = loaded.image.as_ref().map_or(String::new(), |b| {
                format!("  {}\u{00d7}{}", b.src.0, b.src.1)
            });
            vec![banner(&format!("{kind}{dims}{size}"), cols)]
        }
        Format::Extract { via } => {
            out.push(banner(
                &format!(
                    "text extract via {} — press o to open the real file",
                    via.bin()
                ),
                cols,
            ));
            // An extract has no `md::syntax` language of its own — it is
            // prose lifted out of a PDF or a Word doc, not source.
            numbered(text, cols, "", t.ink, t.text_muted, ws)
        }
        // A pane too narrow for two honest columns answers `None` and the
        // unified rung takes it — the toggle is a request, not a promise the
        // width can always keep.
        Format::Diff => {
            let (lines, found) = match split
                .then(|| super::diffsplitdraw::lines(text, cols))
                .flatten()
            {
                Some(pair) => pair,
                None => diff_lines(text, cols, ws),
            };
            marks = shifted(found, out.len());
            lines
        }
        Format::Markdown if !raw => {
            let (lines, found, pics) = super::mdrung::rung(text, &prepared.map, cols, reveal);
            marks = shifted(found, out.len());
            // A banner above the render (a truncation notice) shifts every
            // row under it, pictures included.
            let top = out.len();
            pictures = pics
                .into_iter()
                .map(|p| crate::chatmd::Picture {
                    row: p.row + top,
                    ..p
                })
                .collect();
            lines
        }
        Format::Csv { delim } if !raw => super::csv::lines(text, delim, cols),
        Format::Text => prepared.map.mapped(super::plainrung::unnumbered(
            text,
            cols,
            t.ink,
            t.text_muted,
            ws,
        )),
        // Fix 1: `Code`/`Data` used to reach here with no `lang`, which is
        // why every character painted the same `ink` regardless of what the
        // lexer would have called it. `Format::lang` is `""` for every other
        // rung that lands here (raw `Markdown`/`Csv`), so their behaviour is
        // unchanged.
        _ => prepared
            .map
            .mapped(numbered(text, cols, format.lang(), t.ink, t.text_muted, ws)),
    };
    // A file with nothing in it renders as an empty pane, which is
    // indistinguishable from one that failed to render — and from one still
    // loading, since the "loading…" banner is gone by then. Two rungs are
    // exempt: the opaque card is ABOUT a file rather than the file, and an
    // image's bytes are a picture drawn on the paint layer — both have plenty
    // to say with no text at all, and claiming a photo is "empty" is simply
    // false.
    let wordless = matches!(format, Format::Opaque { .. } | Format::Image { .. });
    if text.trim().is_empty() && !wordless {
        out.push(banner("this file is empty", cols));
        // An empty text file is still one to write in: its one blank line is kept to stand on.
        if !super::editable::typed_in(format, raw) {
            return (out, marks, Vec::new());
        }
    }
    out.extend(body);
    (out, marks, pictures)
}

#[cfg(test)]
#[path = "lines_tests.rs"]
mod tests;
