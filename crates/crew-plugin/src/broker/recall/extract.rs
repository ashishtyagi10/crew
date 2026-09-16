//! Text in, remembered things out: the file paths a turn talked about and the
//! words it kept using.
//!
//! No model call, on purpose. This runs after EVERY turn, on both sides of it,
//! and a memory that costs a completion to write is a memory that gets turned
//! off. Frequency plus a stop list is a blunt instrument that still puts
//! `linecap`, `swarm` and `crates/crew-app/src/nav.rs` in the graph, and the
//! ranking downstream ([`super::query`]) is what decides whether a blunt
//! topic ever gets shown.

/// Words that carry no topic. Short ones are excluded by length, so this is
/// only the long-and-common: the ones that would otherwise link every turn to
/// every other turn.
const STOP: &[&str] = &[
    "about",
    "after",
    "again",
    "also",
    "another",
    "answer",
    "anything",
    "back",
    "because",
    "been",
    "before",
    "being",
    "both",
    "call",
    "came",
    "could",
    "crew",
    "does",
    "doing",
    "done",
    "down",
    "each",
    "else",
    "even",
    "ever",
    "every",
    "file",
    "files",
    "first",
    "from",
    "give",
    "going",
    "good",
    "have",
    "having",
    "here",
    "into",
    "just",
    "keep",
    "know",
    "last",
    "like",
    "line",
    "lines",
    "long",
    "look",
    "made",
    "make",
    "many",
    "more",
    "most",
    "much",
    "must",
    "need",
    "never",
    "next",
    "note",
    "nothing",
    "only",
    "other",
    "over",
    "part",
    "please",
    "really",
    "right",
    "said",
    "same",
    "says",
    "see",
    "should",
    "show",
    "side",
    "since",
    "some",
    "something",
    "still",
    "such",
    "sure",
    "take",
    "tell",
    "than",
    "that",
    "them",
    "then",
    "there",
    "these",
    "they",
    "thing",
    "things",
    "think",
    "this",
    "those",
    "through",
    "time",
    "under",
    "until",
    "used",
    "using",
    "very",
    "want",
    "well",
    "were",
    "what",
    "when",
    "where",
    "which",
    "while",
    "will",
    "with",
    "without",
    "work",
    "would",
    "your",
];

/// Topics returned per side of a turn. Enough to describe what it was about,
/// few enough that one chatty turn cannot dominate the graph.
const TOPICS_MAX: usize = 8;
/// Paths returned per side. Same reason.
const PATHS_MAX: usize = 6;
/// Longest path kept — past this it is a paste, not a reference.
const PATH_MAX_LEN: usize = 120;

const CODE_EXT: &[&str] = &[
    ".rs", ".toml", ".md", ".json", ".yml", ".yaml", ".sh", ".ps1", ".txt", ".lock", ".py", ".ts",
    ".tsx", ".js", ".html", ".css",
];

/// The file paths `text` refers to, in first-seen order, deduplicated.
pub(crate) fn paths(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in text.split_whitespace() {
        // Trailing punctuation is prose, not path: a reference ends in a
        // name. Leading `./` and `_` are path, so the two ends differ.
        let t = raw
            .trim_start_matches(|c: char| !(c.is_alphanumeric() || "/._".contains(c)))
            .trim_end_matches(|c: char| !c.is_alphanumeric());
        if t.len() < 3 || t.len() > PATH_MAX_LEN {
            continue;
        }
        let looks_like_path =
            (t.contains('/') && !t.contains("//")) || CODE_EXT.iter().any(|e| t.ends_with(e));
        if !looks_like_path || t.starts_with("http") {
            continue;
        }
        let t = t.to_owned();
        if !out.iter().any(|p| p.eq_ignore_ascii_case(&t)) {
            out.push(t);
        }
        if out.len() == PATHS_MAX {
            break;
        }
    }
    out
}

/// Pages kept from one turn. A search answers with eight results; what the
/// turn CITED is the interesting few, and a model that pasted all eight was
/// listing, not reading.
const URLS_MAX: usize = 4;

/// Longer than this and it is a tracking blob, not something to recall.
const URL_MAX_LEN: usize = 300;

/// The http(s) URLs in `text`.
///
/// WHY this is separate from [`paths`], which has always skipped anything
/// starting with `http`: a URL is not a file, and until crew could reach the
/// web there was nothing to do with one but drop it. Now that `sys:fetch` and
/// `sys:search` exist, the page a turn cited is the most specific thing that
/// turn knows — "we read THIS" outlives "we talked about that".
///
/// Taken from the turn's text rather than from the tool call, which is not
/// the cheap way round but the honest one: `sys:search` tells the model to
/// cite the URL it used, so what lands here is what the answer actually stood
/// on, not every page a run happened to open and discard.
pub(crate) fn urls(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in text.split_whitespace() {
        let Some(start) = raw.find("http") else {
            continue;
        };
        let t = &raw[start..];
        if !(t.starts_with("http://") || t.starts_with("https://")) {
            continue;
        }
        // Prose punctuation that ends a sentence, not a URL. A trailing `/`
        // stays: it is the one mark that is part of the address.
        let t = t.trim_end_matches(|c: char| ".,;:!?\u{201d}\u{2019}\")]}>'".contains(c));
        // Shorter than this is a scheme and not much else.
        if t.len() < "https://a.bc".len() || t.len() > URL_MAX_LEN {
            continue;
        }
        if !out.iter().any(|u| u == t) {
            out.push(t.to_owned());
        }
        if out.len() == URLS_MAX {
            break;
        }
    }
    out
}

/// The topic words of `text`: lowercased, four letters or more, off the stop
/// list, ranked by how often they appear and then by where they first did.
pub(crate) fn topics(text: &str) -> Vec<String> {
    let mut seen: Vec<(String, usize, usize)> = Vec::new(); // word, count, first index
    for (i, raw) in text.split_whitespace().enumerate() {
        let w: String = raw
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase();
        if w.chars().count() < 4 || w.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if !w
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            continue;
        }
        if STOP.contains(&w.as_str()) {
            continue;
        }
        match seen.iter_mut().find(|(s, _, _)| *s == w) {
            Some((_, n, _)) => *n += 1,
            None => seen.push((w, 1, i)),
        }
    }
    seen.sort_by(|a, b| b.1.cmp(&a.1).then(a.2.cmp(&b.2)));
    seen.into_iter()
        .take(TOPICS_MAX)
        .map(|(w, _, _)| w)
        .collect()
}

#[cfg(test)]
#[path = "extract_tests.rs"]
mod tests;
