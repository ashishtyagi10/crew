//! `sys:fetch` — the one tool that reaches off this machine.
//!
//! WHY: crew's agents could read your disk, run your shell and call whatever
//! MCP servers you had configured, and could not read a web page. "What does
//! the docs page say about X", "check the release notes for that version",
//! "read the issue I linked" — all of it needed `sys:run curl` (the blank
//! cheque, gated as irreversible) or a server you had to install first. Every
//! agentic tool worth copying has a fetch; Grok's whole trick is that the
//! model can see what is live.
//!
//! Bounded on every axis that matters: GET only (no body, no headers a caller
//! chooses), http(s) only, redirects followed to a limit, a deadline, a size
//! cap, and HTML reduced to text before it reaches the model — a raw page is
//! mostly script and markup, and a model paying by the token should not read
//! any of it.
//!
//! One guard is not about size: a URL naming localhost or a private address
//! is refused. The broker sits inside your network, where "fetch this URL"
//! reaches printers, routers and cloud metadata endpoints that trust anyone
//! who can talk to them. The agent asking is usually not an attacker; the
//! page that told it to ask might be.
use std::time::Duration;

/// Chars of extracted text returned. Past this the tail is cut with a marker
/// — a long page is a source to quote from, not a document to carry.
pub(crate) const TEXT_CAP: usize = 24 * 1024;
/// Bytes read off the wire, before extraction.
const BYTES_CAP: usize = 2 * 1024 * 1024;
/// How long the whole fetch may take.
const TIMEOUT: Duration = Duration::from_secs(20);
/// Redirects followed.
const REDIRECTS: usize = 5;

/// Fetch `url` and return its readable text.
pub(crate) fn fetch(url: &str) -> Result<String, String> {
    let (kind, body) = raw(url)?;
    Ok(readable_body(&kind, body))
}

/// A body as the model should read it: markup reduced to prose, the whole
/// thing capped.
///
/// Named rather than inlined into [`fetch`] so the tests can drive it over a
/// loopback server — which [`check`] refuses to let `fetch` itself reach, and
/// should.
fn readable_body(kind: &str, body: String) -> String {
    let text = match kind.contains("html") || body.trim_start().starts_with('<') {
        true => super::sysfetchtext::readable(&body),
        false => body,
    };
    super::sysfetchtext::capped(&text, TEXT_CAP)
}

/// The body as it arrived, with its content type, for the one caller that
/// needs the MARKUP rather than the prose: [`super::syssearch`] reads result
/// hrefs out of it, and `readable` would have thrown those away.
///
/// Same guard, client, deadline and size cap as [`fetch`] — there is one door
/// off this machine and this is the inside of it.
///
/// Runs on a thread of its own with a runtime of its own: tool calls arrive
/// from inside the swarm's runtime, where a nested `block_on` is a panic
/// rather than a wait (the same reason `toolchoice` spawns).
pub(crate) fn raw(url: &str) -> Result<(String, String), String> {
    let url = check(url)?;
    std::thread::scope(|s| {
        s.spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("fetch: no runtime: {e}"))?;
            rt.block_on(get(&url))
        })
        .join()
        .unwrap_or_else(|_| Err("fetch: the request panicked".into()))
    })
}

/// The URL, if it is one we will ask for.
fn check(url: &str) -> Result<String, String> {
    let url = url.trim();
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .ok_or_else(|| format!("fetch: only http(s) URLs \u{2014} got \u{201c}{url}\u{201d}"))?;
    let host = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .rsplit('@')
        .next()
        .unwrap_or_default()
        .split(':')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if host.is_empty() {
        return Err("fetch: no host in the URL".into());
    }
    if is_private(&host) {
        return Err(format!(
            "fetch: refused \u{2014} {host} is on this machine or this network, \
             and a page crew reads must not be able to reach it"
        ));
    }
    Ok(url.to_string())
}

/// Hosts the broker will not fetch: itself, its network, and the addresses
/// cloud providers answer credentials on.
fn is_private(host: &str) -> bool {
    if host == "localhost" || host.ends_with(".localhost") || host.ends_with(".internal") {
        return true;
    }
    let octets: Vec<u8> = host
        .split('.')
        .filter_map(|p| p.parse::<u8>().ok())
        .collect();
    if host.contains(':') || host.starts_with('[') {
        return true; // an IPv6 literal: loopback and link-local are not worth parsing for
    }
    match octets.as_slice() {
        [127, ..] | [10, ..] | [0, ..] => true,
        [169, 254, ..] => true, // link-local, and the metadata endpoint
        [192, 168, ..] => true,
        [172, b, ..] if (16..=31).contains(b) => true,
        _ => false,
    }
}

async fn get(url: &str) -> Result<(String, String), String> {
    let client = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .redirect(reqwest::redirect::Policy::limited(REDIRECTS))
        .user_agent("crew/agent-smith")
        .build()
        .map_err(|e| format!("fetch: {e}"))?;
    let res = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("fetch: {e}"))?;
    let status = res.status();
    let kind = res
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !status.is_success() {
        return Err(format!("fetch: {url} answered {status}"));
    }
    let body = res.bytes().await.map_err(|e| format!("fetch: {e}"))?;
    let body = &body[..body.len().min(BYTES_CAP)];
    Ok((kind, String::from_utf8_lossy(body).to_string()))
}

#[cfg(test)]
#[path = "sysfetch_tests.rs"]
mod tests;
