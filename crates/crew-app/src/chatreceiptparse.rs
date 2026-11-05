//! A receipt's head read back into its parts, for `chatreceipthead` to draw:
//! the broker writes it as one line of text (`broker::receipt::headline`),
//! the only shape that crosses the wire, and the per-file counts the line
//! has no room for come from the fenced patch under it.

/// The broker's head line, read back into its parts.
#[derive(Debug, PartialEq)]
pub(crate) struct Head<'a> {
    pub ending: Option<&'a str>,
    pub files: usize,
    pub total: Option<(usize, usize)>,
    pub named: Vec<(&'a str, &'a str)>,
    pub hint: Option<&'a str>,
}

/// `None` for a line that is not a receipt's head in the form
/// `broker::receipt::headline` writes; the card then prints it as before.
pub(crate) fn parse(line: &str) -> Option<Head<'_>> {
    let rest = line.strip_prefix("task #")?;
    let rest = rest.trim_start_matches(|c: char| c.is_ascii_digit());
    let (ending, rest) = match rest.strip_prefix(": ") {
        Some(r) => (None, r),
        None => {
            let (how, r) = rest.strip_prefix(' ')?.split_once(" \u{b7} ")?;
            (Some(how), r)
        }
    };
    let (rest, hint) = match rest.split_once(" \u{2014} ") {
        Some((a, h)) => (a, Some(h)),
        None => (rest, None),
    };
    let (count, list) = rest.split_once(": ")?;
    let (count, total) = match count.split_once(" (+") {
        Some((c, t)) => {
            let (a, d) = t.strip_suffix(')')?.split_once(" \u{2212}")?;
            (c, Some((a.parse().ok()?, d.parse().ok()?)))
        }
        None => (count, None),
    };
    let (n, word) = count.split_once(' ')?;
    if !word.ends_with("changed") {
        return None;
    }
    let named = list
        .split(", ")
        .filter(|item| !is_more(item))
        .map(|item| item.split_once(' '))
        .collect::<Option<Vec<_>>>()?;
    let files = n.parse().ok()?;
    Some(Head {
        ending,
        files,
        total,
        named,
        hint,
    })
}

/// The `+2 more` the broker ends a long list with — a count, not a file.
fn is_more(item: &str) -> bool {
    let n = item.strip_prefix('+').and_then(|r| r.strip_suffix(" more"));
    n.is_some_and(|n| n.parse::<usize>().is_ok())
}

/// Lines added and removed per file in a receipt's fenced patch, by the path
/// its `diff --git … b/<path>` header names.
pub(crate) fn per_file(text: &str) -> Vec<(&str, usize, usize)> {
    let mut out: Vec<(&str, usize, usize)> = Vec::new();
    let patch = text.find("\n```").map_or("", |at| &text[at..]);
    for l in patch.lines() {
        if let Some((_, b)) = l
            .strip_prefix("diff --git ")
            .and_then(|h| h.rsplit_once(" b/"))
        {
            out.push((b, 0, 0));
        } else if let Some(f) = out.last_mut() {
            match l.as_bytes().first() {
                Some(b'+') if !l.starts_with("+++") => f.1 += 1,
                Some(b'-') if !l.starts_with("---") => f.2 += 1,
                _ => {}
            }
        }
    }
    out
}
