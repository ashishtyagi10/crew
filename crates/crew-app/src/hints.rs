//! Hint mode: label everything on screen worth reaching, then press a letter.
//!
//! A terminal's output is full of things you want to *do something with* — the
//! URL a server printed, the file the compiler named, the hash a commit came
//! back as — and the only ways to get at them were the mouse (Cmd+click, or a
//! drag over exactly the right characters) and the scrollback search. Both ask
//! you to leave the keyboard for something the pane is already showing you.
//!
//! So: `Cmd+E` labels them. Every URL, file reference and hash on the focused
//! pane wears a one-letter tag; pressing that letter copies it, and pressing
//! its capital opens it — a URL in the browser, a file in the viewer. Nothing
//! is drawn but cells, so the labels sit on the pane like everything else
//! crew draws, and the mode ends on Esc, on a pick, or on a letter that
//! matches nothing.
//!
//! Labels come off the home row outward, and are handed out from the BOTTOM of
//! the pane up: the last thing a program printed is the thing you almost
//! always want, and it should be the cheapest key to press.
use std::sync::Mutex;

use crew_render::CellView;

/// Label letters, in the order they are handed out.
const ALPHABET: &[u8] = b"asdfjklghqwertyuiopzxcvbnm";

/// What a labelled thing is, which decides what "open" means for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Url,
    Path,
    /// A hex string long enough to be a commit, an object id or a checksum.
    Hash,
}

/// One labelled thing on the pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Target {
    pub row: u16,
    pub col: u16,
    pub text: String,
    pub kind: Kind,
    pub label: String,
}

/// What a keypress did to the mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Press {
    /// A prefix of one or more labels: keep going.
    Pending,
    /// This target, and whether to open it rather than copy it.
    Pick(Box<Target>, bool),
    /// Nothing starts with that: the mode ends rather than sitting there
    /// swallowing keys a pane wanted.
    Miss,
}

/// The live mode: what is labelled and what has been typed at it.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Hints {
    /// The pane whose output is labelled. The mode is a property of one
    /// pane's contents, so it ends the moment the labels could be wrong.
    pub pane: usize,
    pub targets: Vec<Target>,
    typed: String,
}

/// The one live hint mode, held here rather than on the app for the same
/// reason the pointer's hover is: everything that draws it or answers a key
/// for it is somewhere else, and threading it through every scene signature
/// would touch a dozen files to carry a mode that is off almost always.
static MODE: Mutex<Option<Hints>> = Mutex::new(None);

/// Label `pane`'s rows, if there is anything on them to label. Returns
/// whether the mode opened.
pub(crate) fn open(pane: usize, rows: &[Vec<char>]) -> bool {
    let mut h = Hints::scan(rows);
    if let Some(h) = &mut h {
        h.pane = pane;
    }
    let opened = h.is_some();
    *lock() = h;
    opened
}

/// Whether the mode is on — the one question the key router asks before
/// handing a key to a pane.
pub(crate) fn active() -> bool {
    lock().is_some()
}

pub(crate) fn close() {
    *lock() = None;
}

/// Feed a character to the live mode. `None` when it is not on.
pub(crate) fn press(c: char) -> Option<Press> {
    let mut g = lock();
    let out = g.as_mut().map(|h| h.press(c));
    if !matches!(out, Some(Press::Pending)) {
        *g = None;
    }
    out
}

/// Draw the labels over one pane's cells, if that is the labelled pane.
pub(crate) fn mark_pane(cells: &mut Vec<CellView>, pane: usize) {
    if let Some(h) = lock().as_ref().filter(|h| h.pane == pane) {
        h.mark(cells);
    }
}

/// The labels currently on screen, in the order they were handed out — the
/// seam the app-side tests read through, since the mode is a singleton.
#[cfg(test)]
pub(crate) fn labels_snapshot() -> Vec<String> {
    lock()
        .as_ref()
        .map(|h| h.targets.iter().map(|t| t.label.clone()).collect())
        .unwrap_or_default()
}

fn lock() -> std::sync::MutexGuard<'static, Option<Hints>> {
    MODE.lock().unwrap_or_else(|e| e.into_inner())
}

impl Hints {
    /// Label everything on these rows, or `None` when there is nothing to
    /// label — a mode with no targets is worse than no mode, because it eats
    /// the next key you press.
    pub(crate) fn scan(rows: &[Vec<char>]) -> Option<Self> {
        let mut found: Vec<(u16, u16, String, Kind)> = Vec::new();
        for (r, line) in rows.iter().enumerate() {
            let mut spans: Vec<(usize, usize, Kind)> = crate::openurl::url_spans(line)
                .into_iter()
                .map(|(a, b)| (a, b, Kind::Url))
                .collect();
            for (a, b) in crate::pathhl::path_spans(line) {
                // A URL is also, textually, a path with slashes in it. It was
                // found first and it is the more specific answer.
                if !spans.iter().any(|&(x, y, _)| a < y && x < b) {
                    spans.push((a, b, Kind::Path));
                }
            }
            for (a, b) in hash_spans(line) {
                if !spans.iter().any(|&(x, y, _)| a < y && x < b) {
                    spans.push((a, b, Kind::Hash));
                }
            }
            spans.sort_by_key(|&(a, _, _)| a);
            for (a, b, kind) in spans {
                let text: String = line[a..b].iter().collect();
                found.push((r as u16, a as u16, text, kind));
            }
        }
        if found.is_empty() {
            return None;
        }
        // Cheapest labels to the newest output: hand them out from the last
        // row up, and within a row from the left.
        found.sort_by_key(|&(r, c, _, _)| (std::cmp::Reverse(r), c));
        let labels = labels_for(found.len());
        let targets = found
            .into_iter()
            .zip(labels)
            .map(|((row, col, text, kind), label)| Target {
                row,
                col,
                text,
                kind,
                label,
            })
            .collect();
        Some(Self {
            pane: 0,
            targets,
            typed: String::new(),
        })
    }

    /// Feed a typed character. Capitals pick the same label as their lower
    /// case and mean *open* rather than *copy*.
    pub(crate) fn press(&mut self, c: char) -> Press {
        let open = c.is_uppercase();
        self.typed.push(c.to_ascii_lowercase());
        if let Some(t) = self.targets.iter().find(|t| t.label == self.typed) {
            return Press::Pick(Box::new(t.clone()), open);
        }
        match self
            .targets
            .iter()
            .any(|t| t.label.starts_with(&self.typed))
        {
            true => Press::Pending,
            false => Press::Miss,
        }
    }

    /// Draw the labels over `cells`, and lift the ink of everything else so
    /// the tags are what the eye lands on.
    pub(crate) fn mark(&self, cells: &mut Vec<CellView>) {
        let t = crew_theme::theme();
        // The pane's own text leans toward the page so the tags are what the
        // eye lands on — the same wash an unfocused pane wears, for the same
        // reason, and it lasts exactly as long as the mode does.
        crate::spotlight::wash(cells, 0.4);
        let _ = t;
        for target in &self.targets {
            let done = target.label.starts_with(&self.typed);
            for (i, ch) in target.label.chars().enumerate() {
                let col = target.col + i as u16;
                let (fg, bg) = match done && i < self.typed.chars().count() {
                    // The letters you have already pressed read as spent.
                    true => (t.page_bg, t.text_muted),
                    false => (t.page_bg, crate::palette::accent()),
                };
                cells.retain(|c| !(c.row == target.row && c.col == col));
                cells.push(CellView {
                    col,
                    row: target.row,
                    c: ch,
                    fg,
                    bg,
                    bold: true,
                    ..Default::default()
                });
            }
        }
    }
}

/// Spans of hex long enough to be an object id — a commit, a blob, a digest.
fn hash_spans(chars: &[char]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let start = i;
        while i < chars.len() && chars[i].is_ascii_hexdigit() {
            i += 1;
        }
        let long_enough = (7..=64).contains(&(i - start));
        let whole_word =
            (start == 0 || !is_word(chars[start - 1])) && (i >= chars.len() || !is_word(chars[i]));
        // A run of digits is a number — a line count, a byte size, a port —
        // and labelling every one of those buries the hashes.
        let has_letter = chars[start..i].iter().any(|c| c.is_ascii_alphabetic());
        if long_enough && whole_word && has_letter {
            out.push((start, i));
        }
        i = (i + 1).max(start + 1);
    }
    out
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '/'
}

/// `n` distinct labels, shortest first: single letters while they last, then
/// pairs. Never a label that is a prefix of another, or the first press would
/// have to guess whether you were done.
fn labels_for(n: usize) -> Vec<String> {
    let single = ALPHABET.len();
    if n <= single {
        return ALPHABET
            .iter()
            .take(n)
            .map(|&b| (b as char).to_string())
            .collect();
    }
    // With more targets than letters, every label is a pair: mixing lengths
    // would make some single letter a prefix of a pair.
    ALPHABET
        .iter()
        .flat_map(|&a| {
            ALPHABET
                .iter()
                .map(move |&b| format!("{}{}", a as char, b as char))
        })
        .take(n)
        .collect()
}

#[cfg(test)]
#[path = "hints_tests.rs"]
mod hints_tests;
