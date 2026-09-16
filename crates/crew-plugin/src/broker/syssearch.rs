//! `sys:search` — finding the page that `sys:fetch` then reads.
//!
//! WHY: 0.22.28 gave the agents a fetch, and a fetch can only open a URL
//! somebody already knew. Every question that begins "what is the current
//! version of", "which crate does", "did that API change" needs the step
//! BEFORE the fetch, and without it the model answers those from training
//! data — confidently, and as of whenever it stopped reading. This is the
//! smallest honest version of Grok's live reach: find the pages, hand back
//! their real URLs, let the model fetch the ones it wants and cite them.
//!
//! Keyless on purpose. A search that needs an API key is a search most
//! installs do not have, and a feature only the author can run is not a
//! feature — so this asks the no-JavaScript endpoint for HTML, exactly as a
//! reader without scripts would.
//!
//! It adds no reach: [`super::sysfetch::raw`] is the one door off this
//! machine, so the private-address refusal, the redirect limit, the 20 s
//! deadline and the size cap all hold here without being written twice. What
//! this file owns is the query, the shape of the answer, and its size.
use super::syssearchparse::{hits, query_url, Hit};

/// Results returned. Enough to choose from, few enough that the block stays
/// small next to the page the model is about to fetch anyway.
const MAX_HITS: usize = 8;
/// Chars of each snippet kept. A snippet is a reason to open a link, not a
/// substitute for opening it.
const SNIPPET_CAP: usize = 240;

/// Search the web for `q` and return the ranked results as text.
pub(crate) fn search(q: &str) -> Result<String, String> {
    let q = q.trim();
    if q.is_empty() {
        return Err("search: no query \u{2014} pass {\"q\": \"what to look for\"}".into());
    }
    let (_, html) = super::sysfetch::raw(&query_url(q))?;
    Ok(render(q, &hits(&html, MAX_HITS)))
}

/// The block the model reads.
///
/// The URL sits on its own line under each title because it is the part that
/// gets used twice — fetched, then cited — and a line the model can copy
/// whole is a line it copies correctly.
fn render(q: &str, hits: &[Hit]) -> String {
    if hits.is_empty() {
        // Said plainly, because the model's next move differs: a query with
        // no results wants rewording, not a fetch.
        return format!("no results for \u{201c}{q}\u{201d}");
    }
    let mut out = format!(
        "{} result{} for \u{201c}{q}\u{201d} \u{2014} open one with sys:fetch, and cite the URL you used.\n",
        hits.len(),
        if hits.len() == 1 { "" } else { "s" }
    );
    for (i, hit) in hits.iter().enumerate() {
        out.push_str(&format!("\n{}. {}\n   {}\n", i + 1, hit.title, hit.url));
        let snippet = snippet(&hit.snippet);
        if !snippet.is_empty() {
            out.push_str(&format!("   {snippet}\n"));
        }
    }
    out
}

/// One snippet, on one line, under the cap.
///
/// Newlines are flattened rather than kept: a snippet indented under its URL
/// stops looking like one the moment it wraps into the left margin.
fn snippet(s: &str) -> String {
    let flat = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= SNIPPET_CAP {
        return flat;
    }
    let head: String = flat.chars().take(SNIPPET_CAP).collect();
    format!("{}\u{2026}", head.trim_end())
}

#[cfg(test)]
#[path = "syssearch_tests.rs"]
mod tests;
