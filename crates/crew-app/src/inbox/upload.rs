//! A mail sent through Gmail's upload address (`/upload/gmail/v1/…/send`,
//! the `messages.send` reference): the message's own bytes as the body,
//! `message/rfc822`, rather than base64 inside JSON. A forward goes this
//! way, files and all; Google's upload guide calls the JSON way fit for
//! 5 MB or so.
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

impl Gmail {
    /// Send `message` (RFC 2822 bytes); one 401 renews the token and tries
    /// once more.
    pub(crate) fn upload_send(&mut self, message: Vec<u8>) -> Result<(), String> {
        let url = format!("{}/messages/send?uploadType=media", root(&self.api));
        for attempt in 0..2 {
            let token = self.token()?;
            let (http, url, body) = (self.http.clone(), url.clone(), message.clone());
            let (status, reason) = self.rt.block_on(async move {
                let r = http
                    .post(&url)
                    .bearer_auth(token)
                    .header(reqwest::header::CONTENT_TYPE, "message/rfc822")
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
