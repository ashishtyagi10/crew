//! Passes over a mail's finished markdown, where a fault shows only once the
//! whole of it is written: links set side by side, too many headlines, and a
//! picture left as words that only repeats the caption beside it.
use super::plain_of;

/// A picture left as its words (` ▣ alt `) beside a line that says the same
/// is said once: a product grid read with its pictures hidden came out
/// `▣ Wool scarf` over `Wool scarf` for every product (goal
/// 2026-10-21-mail-looks-right, item 7).
pub(super) fn echoless(md: &str) -> String {
    let lines: Vec<&str> = md.lines().collect();
    let said = |l: &str| plain_of(l).trim().to_lowercase();
    let near = |i: usize, step: isize| {
        let mut j = i as isize + step;
        while let Some(l) = usize::try_from(j).ok().and_then(|j| lines.get(j)) {
            if !l.trim().is_empty() {
                return said(l);
            }
            j += step;
        }
        String::new()
    };
    let kept = lines.iter().enumerate().filter(|&(i, l)| {
        let Some(alt) = l.trim().strip_prefix('\u{25a3}').map(said) else {
            return true;
        };
        alt.is_empty() || (near(i, 1) != alt && near(i, -1) != alt)
    });
    let out: Vec<&str> = kept.map(|(_, l)| *l).collect();
    out.join("\n")
}

/// Runs of paragraphs that are each one short link (a footer's `Home`,
/// `Account`, `Orders`; a row of social marks, each in a cell of its own)
/// set on one line, as the mail sets them side by side.
pub(super) fn joined(md: &str) -> String {
    let lone = |p: &str| {
        let words = p
            .strip_prefix('[')
            .and_then(|r| r.split_once("](<"))
            .map(|(w, _)| w);
        !p.contains('\n')
            && p.ends_with(">)")
            && p.matches("](<").count() == 1
            && words.is_some_and(|w| {
                (1..=30).contains(&w.chars().count()) && !w.starts_with(['`', '\u{2197}'])
            })
    };
    // A button alone in its cell, beside more: a calendar's `Yes`, `Maybe`
    // and `No`, a row of a mail's buttons (`anchor`'s capsules).
    let button = |p: &str| {
        !p.contains('\n')
            && p.starts_with("[`")
            && p.ends_with(">)")
            && p.matches("](<").count() == 1
    };
    let mut out: Vec<String> = Vec::new();
    let mut run = None;
    for p in md.split("\n\n") {
        let kind = match () {
            _ if lone(p) => Some(" \u{b7} "),
            _ if button(p) => Some("  "),
            _ => None,
        };
        match (kind, out.last_mut()) {
            (Some(join), Some(last)) if run == Some(join) => {
                last.push_str(join);
                last.push_str(p);
            }
            _ => {
                out.push(p.to_string());
                run = kind;
            }
        }
    }
    out.join("\n\n")
}

/// A mail that sets more than four lines as its largest headings (a
/// promotion, every offer a headline) has them a step down: a page of
/// accent headlines is no outline at all.
pub(super) fn calmed(md: &str) -> String {
    let tops = md.lines().filter(|l| l.starts_with("# ")).count();
    if tops <= 4 {
        return md.to_string();
    }
    let lines: Vec<String> = md
        .lines()
        .map(|l| match l.strip_prefix("# ") {
            Some(rest) => format!("## {rest}"),
            None => l.to_string(),
        })
        .collect();
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_picture_left_as_words_is_dropped_where_its_caption_says_the_same() {
        let md = "\u{25a3} Wool scarf\n\nWool scarf\n$29\n\n\u{25a3} Boots\n\nRain boots";
        let got = super::echoless(md);
        assert_eq!(got, "\nWool scarf\n$29\n\n\u{25a3} Boots\n\nRain boots");
        let after = "**Hat**\n\u{25a3} hat";
        assert_eq!(
            super::echoless(after),
            "**Hat**",
            "the caption above counts too"
        );
    }
}
