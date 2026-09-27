//! The metadata card for a rung that cannot be rendered (Fix 2): what the
//! file is, why, and — when a `stat` succeeded — its size and mtime. Split
//! out of `lines.rs` to keep that file under the file-length budget.
use crate::chatbody::{plain, CardLine};
use crate::viewpane::detect::Opaque;
use crate::viewpane::load::FileMeta;

/// `s` as card lines, word-wrapped to `cols` — a card thirty columns wide
/// used to end `press  o  to open in the default`, its last word cut by the
/// frame with nothing to say so.
fn rows(s: &str, fg: (u8, u8, u8), bold: bool, cols: usize) -> Vec<CardLine> {
    let full: Vec<char> = s.chars().collect();
    crate::chatlayout::wrap_indices(&full, cols.max(1))
        .into_iter()
        .map(|(a, b)| full[a..b].iter().map(|&c| plain(c, fg, bold)).collect())
        .collect()
}

/// `parts` joined by ` · ` on as few rows as fit, breaking only BETWEEN
/// them: word-wrapped as one string, a narrow card read `modified 3h` / `ago`.
/// A part wider than the card still wraps on its own words.
fn segment_rows(parts: &[String], fg: (u8, u8, u8), cols: usize) -> Vec<CardLine> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for p in parts {
        let joined = format!("{cur}  \u{00b7}  {p}");
        match cur.is_empty() {
            true => cur = p.clone(),
            false if crate::chatwidth::str_w(&joined) <= cols => cur = joined,
            false => out.extend(rows(
                &std::mem::replace(&mut cur, p.clone()),
                fg,
                false,
                cols,
            )),
        }
    }
    out.extend(rows(&cur, fg, false, cols));
    out
}

/// `bytes` in compact units, the same convention `farpane/panelchrome.rs::fmt_size`
/// uses for directory listings — duplicated locally rather than exported
/// across a module boundary for one function used by only one caller there.
pub(crate) fn fmt_size(bytes: u64) -> String {
    const UNITS: [char; 4] = ['K', 'M', 'G', 'T'];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut v = bytes as f64 / 1024.0;
    let mut i = 0;
    while v >= 1024.0 && i + 1 < UNITS.len() {
        v /= 1024.0;
        i += 1;
    }
    if v < 10.0 {
        format!("{v:.1}{}", UNITS[i])
    } else {
        format!("{v:.0}{}", UNITS[i])
    }
}

/// `modified` as `chattime`'s own relative-time convention ("3h ago") rather
/// than a fresh date format invented for one card.
fn mtime_str(modified: Option<std::time::SystemTime>) -> String {
    let ms = modified
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64);
    match ms {
        Some(ms) => crate::chattime::rel_time(&ms.to_string(), crate::chattime::unix_now_ms())
            .unwrap_or_else(|| "unknown".into()),
        None => "unknown".into(),
    }
}

/// The metadata card for a rung that cannot be rendered: what it is, why, and
/// — Fix 2 — its size and mtime when a `stat` produced them (`meta` is
/// `None` only for the `Unreadable` rung, which never got that far).
pub(crate) fn opaque_card(why: Opaque, meta: Option<&FileMeta>, cols: usize) -> Vec<CardLine> {
    let t = crew_theme::theme();
    let head = match why {
        Opaque::Binary => "binary file — nothing to render".to_string(),
        Opaque::NotUtf8 => "not valid UTF-8 — nothing to render".to_string(),
        Opaque::NoExtractor(e) => format!("no extractor: install {}", e.install_hint()),
        Opaque::Unreadable => "cannot read this file — nothing to render".to_string(),
    };
    let kind = match why {
        Opaque::Binary => "binary",
        Opaque::NotUtf8 => "not UTF-8",
        Opaque::NoExtractor(_) => "no extractor",
        Opaque::Unreadable => "unreadable",
    };
    let mut lines = rows(&head, t.ink, true, cols);
    lines.push(Vec::new());
    if let Some(m) = meta {
        let parts = [
            kind.to_string(),
            fmt_size(m.size),
            format!("modified {}", mtime_str(m.modified)),
        ];
        lines.extend(segment_rows(&parts, t.text_muted, cols));
    }
    // The key in the accent, the way every other key crew names is drawn —
    // not padded with spaces to stand in for a keycap (`press  o  to`).
    let mut hint = rows("o opens it in the default app", t.text_muted, false, cols);
    if let Some(key) = hint.first_mut().and_then(|l| l.first_mut()) {
        *key = plain('o', crate::palette::accent(), true);
    }
    lines.extend(hint);
    lines
}

#[cfg(test)]
#[path = "metacard_tests.rs"]
mod tests;
