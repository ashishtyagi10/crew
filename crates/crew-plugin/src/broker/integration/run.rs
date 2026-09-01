//! Sending the request an integration built, and handing the answer back to the agent.
//!
//! Two things here are deliberate and neither is about HTTP.
//!
//! **A thread, not the caller's runtime.** `Tools::call` is synchronous, and it is called from
//! both engines — including from inside the swarm's tokio runtime, where building a second
//! runtime panics outright. So the request runs on a thread of its own with a small
//! current-thread runtime, and the caller blocks on the join. One thread per tool call is
//! nothing next to the network round trip it is waiting for.
//!
//! **A failure is an answer.** A 404, a timeout and a refused credential all come back as text
//! the agent reads and can act on, never as a task failure: `Tools::call`'s `Err` is shown to
//! the model, and the model is the one that can decide to try something else.
use super::request::Req;

/// Response text an agent is handed. The same budget a tool RESULT hop carries: enough to be
/// useful, bounded because a JSON API can answer with a megabyte.
pub(crate) const CAP: usize = 64 * 1024;

/// Send `req` and return its body. `Err` is a message for the agent.
pub(crate) fn send(req: Req) -> Result<String, String> {
    let timeout = std::time::Duration::from_millis(
        std::env::var("CREW_HTTP_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(120_000),
    );
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("could not start the HTTP runtime: {e}"))?;
        rt.block_on(perform(req, timeout))
    })
    .join()
    .map_err(|_| "the HTTP call panicked".to_string())?
}

async fn perform(req: Req, timeout: std::time::Duration) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(timeout)
        .build()
        .map_err(|e| format!("could not build an HTTP client: {e}"))?;
    let method = reqwest::Method::from_bytes(req.method.as_bytes())
        .map_err(|_| format!("{:?} is not an HTTP method", req.method))?;
    let mut b = client.request(method, &req.url);
    for (k, v) in &req.headers {
        b = b.header(k, v);
    }
    if let Some(body) = req.body {
        b = b.body(body);
    }
    let res = b
        .send()
        .await
        .map_err(|e| format!("the request to {} failed: {e}", req.url))?;
    let status = res.status();
    let text = res
        .text()
        .await
        .map_err(|e| format!("could not read the response: {e}"))?;
    let text = clip(&text);
    // The BODY of an error is the useful half — an API that refuses says why in it — so a
    // failing status returns the status AND what came with it.
    match status.is_success() {
        true => Ok(text),
        false => Err(format!("{status}: {text}")),
    }
}

/// Trim to [`CAP`] on a character boundary, saying so.
fn clip(text: &str) -> String {
    if text.len() <= CAP {
        return text.to_string();
    }
    let mut end = CAP;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n[response truncated at {CAP} bytes]", &text[..end])
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
