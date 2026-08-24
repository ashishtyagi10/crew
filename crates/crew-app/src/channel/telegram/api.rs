//! The real Bot API over HTTPS. Only constructed when a token exists, so a crew with no token
//! never opens a socket.
use super::{TelegramApi, Update};

/// Long-poll timeout, seconds. Telegram holds the request open this long when idle, which is
/// what keeps this from being a busy loop against their servers.
const POLL_SECS: u64 = 25;

/// Bot API client. Blocking from the caller's point of view: it owns a small current-thread
/// runtime, because the daemon's serve loop is synchronous and the winit thread must never see
/// any of this.
pub(crate) struct HttpApi {
    token: String,
    rt: tokio::runtime::Runtime,
    http: reqwest::Client,
}

impl HttpApi {
    pub(crate) fn new(token: String) -> Self {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("current-thread runtime");
        Self {
            token,
            rt,
            http: reqwest::Client::new(),
        }
    }

    fn url(&self, method: &str) -> String {
        format!("https://api.telegram.org/bot{}/{method}", self.token)
    }
}

impl TelegramApi for HttpApi {
    fn get_updates(&self, offset: i64) -> Result<Vec<Update>, String> {
        let url = self.url("getUpdates");
        let body = serde_json::json!({
            "offset": offset,
            "timeout": POLL_SECS,
            "allowed_updates": ["message"],
        });
        let v: serde_json::Value = self.rt.block_on(async {
            self.http
                .post(&url)
                .json(&body)
                // A little longer than the long-poll, so a healthy idle poll is never mistaken
                // for a hung one.
                .timeout(std::time::Duration::from_secs(POLL_SECS + 10))
                .send()
                .await
                .map_err(|e| e.to_string())?
                .json()
                .await
                .map_err(|e| e.to_string())
        })?;
        Ok(parse_updates(&v))
    }

    fn send_message(&self, chat_id: i64, text: &str) -> Result<(), String> {
        let url = self.url("sendMessage");
        let body = serde_json::json!({ "chat_id": chat_id, "text": text });
        self.rt.block_on(async {
            let r = self
                .http
                .post(&url)
                .json(&body)
                .timeout(std::time::Duration::from_secs(30))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            if r.status().is_success() {
                Ok(())
            } else {
                Err(format!("telegram sendMessage: HTTP {}", r.status()))
            }
        })
    }
}

/// Pull the message updates out of a `getUpdates` response, skipping anything that is not a text
/// message (joins, edits, photos). Tolerant by design: one unexpected update shape must not stop
/// the ones around it from being delivered.
pub(crate) fn parse_updates(v: &serde_json::Value) -> Vec<Update> {
    let Some(items) = v.get("result").and_then(|r| r.as_array()) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|u| {
            let update_id = u.get("update_id")?.as_i64()?;
            let msg = u.get("message")?;
            let chat_id = msg.get("chat")?.get("id")?.as_i64()?;
            let text = msg.get("text")?.as_str()?.to_string();
            Some(Update {
                update_id,
                chat_id,
                text,
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "api_tests.rs"]
mod tests;
