//! A `/review` reply drawn as findings, not as a bullet list.
//!
//! The broker checks each `severity — file:line — why` bullet against the
//! tree and recounts the `verdict:` line (`reviewfindings`). The pane used to
//! print all of that as words: the severity a word like any other, the
//! verdict a sentence two rows under the last finding. Here each finding is
//! one row led by a drawn chip in its severity's colour, the citation in path
//! ink and the why wrapped under the citation, and the verdict moves into the
//! card's header as a tally — the two rows it took (and the blank above it)
//! are gone. The bullets are read with the broker's own parser
//! (`crew_plugin::review::finding`), so the pane never disagrees with what
//! was checked.
use crate::chatbody::{plain, CardCell, CardLine, Color};
use crate::segment::{self, Caps};
use crew_plugin::review::{finding, is_verdict, SEVERITIES};

/// A finding line split for drawing: severity, citation, and the why.
struct Finding<'a> {
    sev: &'static str,
    cite: String,
    why: &'a str,
}

/// The severity's colour: a blocker in the redder of the bell and the
/// palette's red, a warn in the amber a waiting toast wears, a nit quiet.
/// Not the bell alone: on prism the bell IS amber and the two chips read as
/// one; not ansi red alone: on blossom it is a peach beside the warn.
pub(crate) fn sev_color(sev: &str) -> Color {
    let t = crew_theme::theme();
    match sev {
        "blocker" => [t.bell, t.ansi[1]]
            .into_iter()
            .max_by_key(|c| c.0 as i32 - c.1.max(c.2) as i32)
            .unwrap_or(t.bell),
        "warn" => t.ansi[11],
        _ => t.text_muted,
    }
}

fn split(line: &str) -> Option<Finding<'_>> {
    let (sev, file, at) = finding(line)?;
    let cite = at.map_or(file.clone(), |n| format!("{file}:{n}"));
    // The why starts after the citation as written (backticks, `:12` and
    // all), past the separators the parser skipped before it.
    let from = line.find(&file)? + file.len();
    let rest = &line[from..];
    let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let why = rest[end..].trim_start_matches(|c: char| c.is_whitespace() || "—–-:,".contains(c));
    Some(Finding { sev, cite, why })
}

/// Whether `text` is a checked review: finding bullets closed by a verdict.
/// Both are asked for, so a bullet that happens to start `warn —` in any
/// other answer stays prose.
pub(crate) fn is_review(text: &str) -> bool {
    text.lines().any(is_verdict) && text.lines().any(|l| finding(l).is_some())
}

/// The header's tally, ` · 1 blocker · 2 warn` worst first, each count in
/// its severity's ink; nothing for a card that is not a review.
pub(crate) fn tally_cells(text: &str) -> Vec<CardCell> {
    let muted = crew_theme::theme().text_muted;
    let mut out = Vec::new();
    for sev in SEVERITIES.iter().filter(|_| is_review(text)) {
        let n = text.lines().filter_map(finding).filter(|f| f.0 == *sev);
        let word = format!(" \u{b7} {} {sev}", n.count());
        if !word.contains(" 0 ") {
            let ink = |i| if i < 3 { muted } else { sev_color(sev) };
            out.extend(
                word.chars()
                    .enumerate()
                    .map(|(i, c)| plain(c, ink(i), i >= 3)),
            );
        }
    }
    out
}

/// The why's inline markdown (`code`, **bold**) as cells on one line; the
/// row wraps them itself, under the citation.
fn why_cells(why: &str, fg: Color) -> Vec<CardCell> {
    let muted = crew_theme::theme().text_muted;
    crate::md::render_chat(why, 4096)
        .iter()
        .flat_map(|l| {
            l.spans
                .iter()
                .flat_map(move |s| crate::chatmdcells::span_cells(s, l.kind, fg, muted))
        })
        .map(|mut c| {
            c.src = None;
            c
        })
        .collect()
}

/// One finding's rows in `width` columns: chip, citation, why, the why's
/// continuation hanging under the citation.
fn rows(f: &Finding, width: usize, fg: Color) -> Vec<CardLine> {
    // Every chip as wide as the widest severity, so the citations start in
    // one column and the eye runs down them.
    let w = SEVERITIES.iter().map(|s| s.len()).max().unwrap_or(0);
    let label = format!("{:^w$}", f.sev);
    let chip = segment::to_card(&segment::badge_on(&label, sev_color(f.sev), Caps::BOTH));
    let hang = chip.len() + 1;
    let mut cells = chip;
    cells.push(plain(' ', fg, false));
    // A citation wider than the row gives up its LEFT end, as a receipt's
    // path does: `…/src/lib.rs:42` still names the file and the line.
    let cite = crate::chatreceiptcells::tail_fit(&f.cite, width.saturating_sub(hang).max(8));
    let path = crate::chatink::link_color();
    cells.extend(cite.chars().map(|c| plain(c, path, false)));
    cells.push(plain(' ', fg, false));
    cells.extend(why_cells(f.why, fg));
    // The chip and the citation never break: they wrap as one word.
    let words: Vec<char> = cells
        .iter()
        .enumerate()
        .map(|(i, c)| {
            if i < hang + cite.chars().count() {
                'x'
            } else {
                c.c
            }
        })
        .collect();
    let first = crate::chatlayout::wrap_indices(&words, width)[0].1;
    let mut out = vec![std::iter::once(plain(' ', fg, false))
        .chain(cells[..first].iter().cloned())
        .collect::<CardLine>()];
    let mut s = first;
    while words.get(s) == Some(&' ') {
        s += 1;
    }
    if s < words.len() {
        let pad = if width > hang + 8 { hang } else { 0 };
        for (a, b) in crate::chatlayout::wrap_indices(&words[s..], width - pad) {
            let mut row = vec![plain(' ', fg, false); 1 + pad];
            row.extend(cells[s + a..s + b].iter().cloned());
            out.push(row);
        }
    }
    out
}

/// A review card's body for a card `cols` wide: finding rows, the verdict
/// dropped (the header carries it), anything else laid out as markdown.
pub(crate) fn card_lines(
    text: &str,
    cols: usize,
    fg: Color,
    cwd: Option<&std::path::Path>,
) -> Vec<CardLine> {
    let width = cols.saturating_sub(2).max(1);
    let mut out = Vec::new();
    let mut prose: Vec<&str> = Vec::new();
    let flush = |prose: &mut Vec<&str>, out: &mut Vec<CardLine>| {
        let chunk = prose.join("\n");
        if !chunk.trim().is_empty() {
            out.extend(crate::chatbody::body_lines_at(
                chunk.trim(),
                cols,
                fg,
                false,
                cwd,
            ));
        }
        prose.clear();
    };
    for line in text.lines() {
        if is_verdict(line) {
            continue;
        }
        match split(line) {
            Some(f) => {
                flush(&mut prose, &mut out);
                out.extend(rows(&f, width, fg));
            }
            None => prose.push(line),
        }
    }
    flush(&mut prose, &mut out);
    out
}

#[cfg(test)]
#[path = "chatfinding_tests.rs"]
mod tests;
