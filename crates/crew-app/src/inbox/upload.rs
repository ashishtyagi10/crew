//! A mail sent through Gmail's upload address (`/upload/gmail/v1/…/send`,
//! the `messages.send` reference): the message's own bytes, `message/rfc822`,
//! rather than base64 inside JSON. A mail with files goes this way; Google's
//! upload guide calls the plain way fit for 5 MB or so. A reply's thread
//! rides beside it as the upload's JSON part (`uploadType=multipart`); a
//! mail with no thread is sent bare (`uploadType=media`).
use std::time::Duration;

use super::api::Gmail;

/// Long enough for 25 MB of files on a slow line.
const UPLOAD: Duration = Duration::from_secs(180);

/// The upload root for an API root: `/upload` before its path.
pub(crate) fn root(api: &str) -> String {
    let path = api
        .find("://")
        .and_then(|s| api[s + 3..].find('/').map(|p| s + 3 + p))
        .unwrap_or(api.len());
    format!("{}/upload{}", &api[..path], &api[path..])
}

/// The upload's kind, its `Content-Type` and its body: the bare message, or
/// the thread's JSON and the message as two parts of a `multipart/related`.
fn body(message: Vec<u8>, thread: Option<&str>) -> (&'static str, String, Vec<u8>) {
    let Some(id) = thread else {
        return ("media", "message/rfc822".into(), message);
    };
    // `=_crew_up` never begins the message's own boundaries (`=_crew_<n>`).
    let b = "=_crew_upload";
    let meta = serde_json::json!({ "threadId": id });
    let mut out = format!(
        "--{b}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{meta}\r\n--{b}\r\nContent-Type: message/rfc822\r\n\r\n"
    )
    .into_bytes();
    out.extend(message);
    out.extend(format!("\r\n--{b}--\r\n").into_bytes());
    ("multipart", format!("multipart/related; boundary={b}"), out)
}

impl Gmail {
    /// Send `message` (RFC 2822 bytes), in thread `thread` when there is
    /// one; one 401 renews the token and tries once more.
    pub(crate) fn upload_send(
        &mut self,
        message: Vec<u8>,
        thread: Option<&str>,
    ) -> Result<(), String> {
        let (kind, mime, message) = body(message, thread);
        let url = format!("{}/messages/send?uploadType={kind}", root(&self.api));
        for attempt in 0..2 {
            let token = self.token()?;
            let (http, url, body) = (self.http.clone(), url.clone(), message.clone());
            let mime = mime.clone();
            let (status, reason) = self.rt.block_on(async move {
                let r = http
                    .post(&url)
                    .bearer_auth(token)
                    .header(reqwest::header::CONTENT_TYPE, mime)
                    .body(body)
                    .timeout(UPLOAD)
                    .send()
                    .await
                    .map_err(|e| format!("could not reach Gmail: {}", e.without_url()))?;
                let status = r.status().as_u16();
                let v: serde_json::Value = r.json().await.unwrap_or_default();
                let reason = v["error"]["message"].as_str().unwrap_or("no reason given");
                Ok::<_, String>((status, reason.to_string()))
            })?;
            match status {
                200..=299 => return Ok(()),
                401 if attempt == 0 => self.access = None,
                401 => return Err("Gmail refused the sign-in (401)".into()),
                _ => {
                    return Err(crate::oauth::short_reason(&format!(
                        "Gmail {status}: {reason}"
                    )))
                }
            }
        }
        unreachable!("the second attempt always returns")
    }
}
