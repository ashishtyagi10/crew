//! The metadata card for a rung that cannot be rendered (Fix 2): what the
//! file is, why, and — when a `stat` succeeded — its size and mtime. Split
//! out of `lines.rs` to keep that file under the file-length budget.
use crate::chatbody::{plain, CardLine};
use crate::viewpane::detect::Opaque;
use crate::viewpane::load::FileMeta;

fn row(s: &str, fg: (u8, u8, u8), bold: bool) -> CardLine {
    s.chars().map(|c| plain(c, fg, bold)).collect()
}

/// `bytes` in compact units, the same convention `farpane/render.rs::fmt_size`
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
        Opaque::Unreadable => "can't read this file — nothing to render".to_string(),
    };
    let kind = match why {
        Opaque::Binary => "binary",
        Opaque::NotUtf8 => "not UTF-8",
        Opaque::NoExtractor(_) => "no extractor",
        Opaque::Unreadable => "unreadable",
    };
    let mut lines = vec![row(&head, t.ink, true), Vec::new()];
    if let Some(m) = meta {
        lines.push(row(
            &format!(
                "{kind}  \u{00b7}  {}  \u{00b7}  modified {}",
                fmt_size(m.size),
                mtime_str(m.modified)
            ),
            t.text_muted,
            false,
        ));
    }
    lines.push(row(
        "press  o  to open in the default app",
        t.text_muted,
        false,
    ));
    lines
        .into_iter()
        .map(|mut l| {
            l.truncate(cols.max(1));
            l
        })
        .collect()
}

#[cfg(test)]
#[path = "metacard_tests.rs"]
mod tests;
