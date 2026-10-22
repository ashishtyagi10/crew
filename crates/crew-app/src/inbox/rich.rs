//! One message ready to read: its markdown, its folded history, the other
//! version of it, and a line about anything that could not be read (goal
//! 2026-10-20-mail-reader, phase 1).
//!
//! The HTML part is the one shown when there is one — it has the links, the
//! emphasis and the structure — and `v` swaps to the plain part, the escape
//! hatch when a conversion reads wrong (item 4).
use super::{html, plain};

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Rich {
    /// Markdown, for `md::render_chat`.
    pub body: String,
    /// The quoted history, folded until `q`.
    pub quote: Option<String>,
    /// A plain mail's signature, drawn quiet.
    pub sig: Option<String>,
    /// An HTML mail's small print, drawn quiet (`html::Converted::foot`).
    pub foot: Option<String>,
    /// What could not be read: a charset, a cut.
    pub note: Option<String>,
    /// The other version of the same message (`v`).
    pub alt: Option<Box<Rich>>,
    /// The pictures this version draws, by source (`cid:…`, a web address).
    pub pics: Vec<String>,
    /// Pictures from the web it names but does not draw.
    pub web: usize,
    /// This version with the web's pictures drawn too (`i`), when it has any.
    pub with_web: Option<Box<Rich>>,
    /// Pictures written into the mail, by name and payload (`html::inline`).
    pub inline: Vec<(String, String)>,
}

impl Rich {
    /// From a message's HTML.
    pub(crate) fn of_html(html_text: &str, charset: Option<String>) -> Self {
        let build = |web: bool| {
            let c = html::to_markdown(html_text, web);
            let (body, quote) = fold(c.body, c.quote);
            let note = match (&charset, c.cut) {
                (Some(cs), _) => Some(unread(cs)),
                (None, true) => {
                    Some("the rest of this mail is in Gmail \u{2014} o opens it".into())
                }
                (None, false) => None,
            };
            Self {
                body,
                quote,
                foot: c.foot,
                note,
                pics: c.pics,
                web: c.web_left,
                inline: c.inline,
                ..Self::default()
            }
        };
        let bare = build(false);
        Self {
            with_web: (bare.web > 0).then(|| Box::new(build(true))),
            ..bare
        }
    }

    /// From a message's plain text.
    pub(crate) fn of_plain(text: &str, charset: Option<String>) -> Self {
        let s = plain::split(text);
        let (body, quote) = fold(s.body, s.quote);
        Self {
            body,
            quote,
            sig: s.sig,
            note: charset.map(|cs| unread(&cs)),
            ..Self::default()
        }
    }

    /// Everything a reader could unfold, as one markdown text: what agent
    /// smith is handed (hidden text was never in it, item 9).
    pub(crate) fn whole(&self) -> String {
        [
            Some(&self.body),
            self.foot.as_ref(),
            self.quote.as_ref(),
            self.sig.as_ref(),
        ]
        .into_iter()
        .flatten()
        .filter(|s| !s.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join("\n\n")
    }
}

/// History stays folded only under words of the sender's own, and never
/// when it is a forwarded mail: then it is the content.
fn fold(body: String, quote: Option<String>) -> (String, Option<String>) {
    match quote {
        Some(q) if body.trim().is_empty() || html::forwarded(&q) => {
            let all = [body, q].join("\n\n");
            (all.trim().to_string(), None)
        }
        q => (body, q),
    }
}

fn unread(charset: &str) -> String {
    format!("this mail is in {charset}, which crew reads only roughly \u{2014} o opens it in Gmail")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_forward_and_a_bare_quote_stay_open_and_a_reply_folds() {
        let r = Rich::of_html(
            "<p>Thanks!</p><div class=gmail_quote>On Mon, Ann wrote:<blockquote>hi</blockquote></div>",
            None,
        );
        assert_eq!((r.body.as_str(), r.quote.is_some()), ("Thanks\\!", true));
        let f = Rich::of_html(
            "<div class=gmail_quote>---------- Forwarded message ---------<br>hi</div>",
            None,
        );
        assert!(f.quote.is_none() && f.body.contains("hi"), "{f:?}");
        let p = Rich::of_plain("> only a quote", None);
        assert_eq!((p.body.as_str(), p.quote), ("> only a quote", None));
    }

    #[test]
    fn smith_gets_everything_unfoldable_and_an_odd_charset_is_said() {
        let r = Rich::of_plain(
            "ok\n\n-- \nBob\n\nOn Mon, Ann wrote:\n> q",
            Some("koi8-r".into()),
        );
        assert_eq!(r.whole(), "ok\n\nOn Mon, Ann wrote:\n> q\n\nBob");
        assert!(r.note.as_deref().is_some_and(|n| n.contains("koi8-r")));
    }
}

#[cfg(test)]
#[path = "htmlfuzz_tests.rs"]
mod fuzz;
