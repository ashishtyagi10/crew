//! One message laid out in the open thread: who and when, the row about
//! pictures from the web when they are hidden, its words, its folded
//! history, its pictured attachments, its signature, and the quiet lines
//! under it — and where
//! every picture landed, which only the layout can know (goal
//! 2026-10-20-mail-reader, phases 1 and 2).
use super::api::Mail;
use super::draw::wrap;
use super::reader::{row, when, Key};
use super::rich::Rich;
use super::wire::sender_name;
use super::{pics, Reading};
use crate::chatbody::{CardLine, Color};

/// A picture's box in the laid-out thread.
pub(crate) struct Placed {
    pub row: usize,
    pub rows: usize,
    /// What it is stored under ([`pics::key`]).
    pub key: String,
    pub alt: String,
}

/// The thread's lines and the pictures in them, as they are built.
#[derive(Default)]
pub(crate) struct Lay {
    pub lines: Vec<CardLine>,
    pub pics: Vec<Placed>,
}

impl Lay {
    /// Markdown at the layout's width, its pictures placed under `msg`.
    fn md(&mut self, text: &str, k: &Key, fg: Color, msg: &str) {
        let at = self.lines.len();
        let rendered = crate::md::render_chat(text, k.width);
        let (lines, placed) = crate::chatmd::with_pictures(rendered, k.width, fg);
        self.lines.extend(lines);
        self.pics.extend(placed.into_iter().map(|p| Placed {
            row: at + p.row,
            rows: p.rows,
            key: pics::key(msg, &p.src),
            alt: p.alt,
        }));
    }

    fn quiet(&mut self, text: &str, k: &Key) {
        let rows = wrap(text, k.width);
        self.lines
            .extend(rows.iter().map(|l| row(l, k.muted, false)));
    }
}

pub(crate) fn message(lay: &mut Lay, m: &Mail, r: &Reading, k: &Key) {
    // A mail built from its text alone (an older grant's, a test's).
    let own = (m.rich.body.is_empty() && !m.text.is_empty()).then(|| Rich::of_plain(&m.text, None));
    let rich = own.as_ref().unwrap_or(&m.rich);
    let rich = match (r.plain, &rich.alt) {
        (true, Some(alt)) => alt.as_ref(),
        _ => rich,
    };
    let rich = match (r.web, &rich.with_web) {
        (true, Some(w)) => w.as_ref(),
        _ => rich,
    };
    lay.lines.push(Vec::new());
    let who = format!("{} \u{b7} {}", sender_name(&m.from), when(&m.date));
    lay.lines.push(row(&who, k.muted, true));
    // Said where it shows, under the sender, not past the end of the mail.
    if rich.web > 0 && !r.web {
        let s = if rich.web == 1 { "" } else { "s" };
        let line = format!(
            "\u{25a3} {} picture{s} from the web hidden \u{b7} i shows them",
            rich.web
        );
        lay.quiet(&line, k);
    }
    lay.md(&rich.body, k, k.ink, &m.id);
    if let Some(q) = &rich.quote {
        match r.quotes {
            true => lay.md(q, k, k.ink, &m.id),
            false => {
                let n = crate::md::render_chat(q, k.width).len();
                lay.quiet(
                    &format!("\u{b7}\u{b7}\u{b7} {n} lines quoted from earlier replies \u{b7} q shows them"),
                    k,
                );
            }
        }
    }
    let attached = m
        .parts
        .iter()
        .filter(|p| pics::attached(p))
        .take(pics::ATTACHED);
    for p in attached {
        let name = super::html::esc(&p.name);
        let att = p.att.as_deref().unwrap_or_default();
        lay.md(&format!("![{name}](<att:{att}#rows=12>)"), k, k.ink, &m.id);
    }
    if let Some(sig) = &rich.sig {
        lay.lines.push(Vec::new());
        lay.md(sig, k, k.muted, &m.id);
    }
    if let Some(note) = &rich.note {
        lay.quiet(note, k);
    }
    for (n, f) in m.attached.iter().enumerate() {
        let link = format!("{}{}/{n}", super::save::ATTACHMENT, m.id);
        lay.lines.push(file_row(&f.name, f.size, &link, k));
    }
    if m.attached.is_empty() && !m.files.is_empty() {
        lay.quiet(&format!("attached: {}", m.files.join(", ")), k);
    }
}

/// `↓ report.pdf · 1.2 MB`, the name a link that saves and opens it.
fn file_row(name: &str, size: u64, link: &str, k: &Key) -> CardLine {
    let link: std::sync::Arc<str> = link.into();
    let mut line = row("\u{2193} ", k.muted, false);
    line.extend(name.chars().map(|c| crate::chatbody::CardCell {
        link: Some(std::sync::Arc::clone(&link)),
        ..crate::chatbody::plain(c, k.ink, false)
    }));
    line.extend(row(
        &format!("\u{b7} {}", super::save::size(size)),
        k.muted,
        false,
    ));
    line
}
