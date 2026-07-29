//! Rung → `Vec<CardLine>`. Every format lands in the same representation the
//! chat cards use, so `render` is one mapper and each rung is tested as data.
use crate::chatbody::{plain, CardLine};
use crate::viewpane::detect::{Format, Opaque};
use crate::viewpane::load::{Loaded, MAX_VIEW_BYTES};
use crate::viewpane::LoadState;

/// Width of the line-number gutter, digits plus one space.
pub(crate) const GUTTER_W: usize = 6;

fn row(s: &str, fg: (u8, u8, u8), bold: bool) -> CardLine {
    s.chars().map(|c| plain(c, fg, bold)).collect()
}

/// Hard-wrap `text` at `w` display columns, tagging each row with its 1-based
/// source line (continuations repeat it so the gutter can blank them). Lifted
/// unchanged from `mdcache::wrap_source`, which the deleted source half used.
fn wrap(text: &str, w: usize) -> Vec<(usize, Vec<char>)> {
    let mut out = Vec::new();
    for (i, line) in text.split('\n').enumerate() {
        let n = i + 1;
        let chars: Vec<char> = line.chars().collect();
        if w == 0 || chars.is_empty() {
            out.push((n, Vec::new()));
            continue;
        }
        let mut s = 0;
        while s < chars.len() {
            let e = crate::chatwidth::fit_end(&chars, s, w);
            out.push((n, chars[s..e].to_vec()));
            s = e;
        }
    }
    out
}

/// Numbered rows for the gutter rungs.
fn numbered(text: &str, cols: usize, ink: (u8, u8, u8), muted: (u8, u8, u8)) -> Vec<CardLine> {
    let w = cols.saturating_sub(GUTTER_W).max(1);
    let mut out = Vec::new();
    let mut last = 0usize;
    for (n, chars) in wrap(text, w) {
        let mut line: CardLine = if n == last {
            row(&" ".repeat(GUTTER_W), muted, false)
        } else {
            row(&format!("{n:>5} "), muted, false)
        };
        last = n;
        line.extend(chars.iter().map(|c| plain(*c, ink, false)));
        out.push(line);
    }
    out
}

/// `+`/`−` ink for diffs; everything else is body ink.
fn diff_lines(text: &str, cols: usize) -> Vec<CardLine> {
    let t = crew_theme::theme();
    let mut out = Vec::new();
    let w = cols.saturating_sub(GUTTER_W).max(1);
    for (n, chars) in wrap(text, w) {
        let head = chars.first().copied().unwrap_or(' ');
        let fg = match head {
            '+' => t.ansi[2],
            '-' => t.ansi[1],
            '@' => t.ansi[6],
            _ => t.ink,
        };
        let mut line: CardLine = row(&format!("{n:>5} "), t.text_muted, false);
        line.extend(chars.iter().map(|c| plain(*c, fg, false)));
        out.push(line);
    }
    out
}

fn banner(msg: &str, cols: usize) -> CardLine {
    let t = crew_theme::theme();
    let mut s: String = msg.chars().take(cols.max(1)).collect();
    while s.chars().count() < cols {
        s.push(' ');
    }
    row(&s, t.text_muted, false)
}

fn mb(bytes: u64) -> u64 {
    bytes / (1024 * 1024)
}

/// The metadata card for a rung that cannot be rendered.
fn opaque_card(why: Opaque, cols: usize) -> Vec<CardLine> {
    let t = crew_theme::theme();
    let head = match why {
        Opaque::Binary => "binary file — nothing to render".to_string(),
        Opaque::NotUtf8 => "not valid UTF-8 — nothing to render".to_string(),
        Opaque::NoExtractor(e) => format!("no extractor: install {}", e.install_hint()),
    };
    vec![
        row(&head, t.ink, true),
        Vec::new(),
        row("press  o  to open in the default app", t.text_muted, false),
    ]
    .into_iter()
    .map(|mut l| {
        l.truncate(cols.max(1));
        l
    })
    .collect()
}

/// Lines for the pane's current state at `cols` columns. `raw` shows text
/// unrendered (the `s` toggle); it only changes the `Markdown` rung, since
/// every other rung already shows the bytes as they are.
pub(crate) fn for_state(state: &LoadState, raw: bool, cols: usize) -> Vec<CardLine> {
    let t = crew_theme::theme();
    match state {
        LoadState::Loading { .. } => vec![banner("loading…", cols)],
        LoadState::Failed(msg) => vec![row(msg, t.ink, false)],
        LoadState::Ready { format, loaded } => ready_lines(*format, loaded, raw, cols),
    }
}

fn ready_lines(format: Format, loaded: &Loaded, raw: bool, cols: usize) -> Vec<CardLine> {
    let t = crew_theme::theme();
    let mut out = Vec::new();
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
    let body = match format {
        Format::Opaque { why } => opaque_card(why, cols),
        Format::Extract { via } => {
            out.push(banner(
                &format!(
                    "text extract via {} — press o to open the real file",
                    via.bin()
                ),
                cols,
            ));
            numbered(&loaded.text, cols, t.ink, t.text_muted)
        }
        Format::Diff => diff_lines(&loaded.text, cols),
        Format::Markdown if !raw => super::mdrung::lines(&loaded.text, cols),
        Format::Csv { delim } if !raw => super::csv::lines(&loaded.text, delim, cols),
        _ => numbered(&loaded.text, cols, t.ink, t.text_muted),
    };
    out.extend(body);
    out
}

#[cfg(test)]
#[path = "lines_tests.rs"]
mod tests;
