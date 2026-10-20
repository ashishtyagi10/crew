//! A plain-text mail as markdown the reader can draw, without reading it as
//! markdown: every character the sender typed is escaped, so a `*` or a `#`
//! stays itself (goal 2026-10-20-mail-reader, item 5). What is lifted out is
//! the shape plain mail has always had: `>` runs become quote bars, the
//! history under "On … wrote:" is split off to fold, and a signature after
//! the `-- ` line is set apart.
use super::html::{esc, start_safe};

/// Plain mail, split: the words, the quoted history, the signature.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Split {
    pub body: String,
    pub quote: Option<String>,
    pub sig: Option<String>,
}

pub(crate) fn split(text: &str) -> Split {
    let lines: Vec<&str> = text.lines().collect();
    let q = quote_start(&lines);
    let (main, quoted) = lines.split_at(q.unwrap_or(lines.len()));
    // The delimiter is `-- ` (trailing space trimmed by the time it is here);
    // a long tail after one is a mail that quotes a `--`, not a signature.
    let sig = main
        .iter()
        .rposition(|l| l.trim_end() == "--")
        .filter(|&at| main.len() - at <= 15);
    let (words, sig) = match sig {
        Some(at) => (&main[..at], Some(&main[at + 1..])),
        None => (main, None),
    };
    Split {
        body: md(words),
        quote: Some(md(quoted)).filter(|q| !q.is_empty()),
        sig: sig.map(md).filter(|s| !s.is_empty()),
    }
}

/// Where the history starts: an "On … wrote:" line, an Outlook
/// "-----Original Message-----", or a `>` run that lasts to the end.
fn quote_start(lines: &[&str]) -> Option<usize> {
    let quoted = |l: &&str| l.trim_start().starts_with('>');
    for (i, l) in lines.iter().enumerate() {
        let t = l.trim();
        let attribution = t.starts_with("On ") && t.ends_with("wrote:");
        if attribution || t.contains("-----Original Message-----") {
            return Some(i);
        }
        if quoted(l) && lines[i..].iter().all(|l| quoted(l) || l.trim().is_empty()) {
            return Some(i);
        }
    }
    None
}

/// Lines as markdown: escaped, `>` depth kept as quote bars, a line break
/// where the sender broke the line (the reader renders with `render_chat`).
pub(crate) fn md(lines: &[&str]) -> String {
    let mut out: Vec<String> = Vec::new();
    for raw in lines {
        let mut rest = raw.trim_end();
        let mut depth = 0;
        while let Some(r) = rest.trim_start().strip_prefix('>') {
            depth += 1;
            rest = r.strip_prefix(' ').unwrap_or(r);
        }
        let bars = "> ".repeat(depth);
        if rest.trim().is_empty() {
            out.push(bars.trim_end().to_string());
            continue;
        }
        // Leading spaces would be an indented code block: no-break spaces
        // keep the indent without the meaning.
        let lead = rest.len() - rest.trim_start().len();
        let text = format!(
            "{}{}",
            "\u{a0}".repeat(lead),
            start_safe(&esc(rest.trim_start()))
        );
        out.push(format!("{bars}{text}"));
    }
    while out
        .first()
        .is_some_and(|l| l.trim_start_matches('>').trim().is_empty())
    {
        out.remove(0);
    }
    while out
        .last()
        .is_some_and(|l| l.trim_start_matches('>').trim().is_empty())
    {
        out.pop();
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reply_splits_into_words_signature_and_history() {
        let s = split(
            "Sure, *Friday* works.\n# not a heading\n\n-- \nBob\n555-0100\n\nOn Mon, Ann wrote:\n> Can we meet?\n>\n> > older",
        );
        assert_eq!(s.body, "Sure, \\*Friday\\* works.\n\\# not a heading");
        assert_eq!(s.sig.as_deref(), Some("Bob\n555-0100"));
        assert_eq!(
            s.quote.as_deref(),
            Some("On Mon, Ann wrote:\n> Can we meet?\n>\n> > older")
        );
    }

    #[test]
    fn a_trailing_quote_without_attribution_still_folds_and_inline_quotes_do_not() {
        let s = split("> what you said\nmy answer\n> more\n\nthanks");
        assert_eq!(s.quote, None, "a quote answered inline is the reply itself");
        assert!(
            s.body.starts_with("> what you said\nmy answer"),
            "{}",
            s.body
        );
        let s = split("ok!\n\n> earlier\n> mail");
        assert_eq!(s.quote.as_deref(), Some("> earlier\n> mail"));
    }

    #[test]
    fn lists_and_indents_stay_text() {
        assert_eq!(
            md(&["- one", "1. two", "    code?"]),
            "\\- one\n1\\. two\n\u{a0}\u{a0}\u{a0}\u{a0}code?"
        );
    }
}
