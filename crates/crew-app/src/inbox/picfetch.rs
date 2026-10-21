//! Getting a picture's bytes and reading them, carefully: mail is written by
//! strangers, and a picture is the one part of it decoded by a library that
//! will allocate whatever the file's header asks for.
//!
//! - **Decoding** runs under [`image::Limits`] (goal 2026-10-20-mail-reader,
//!   item 13): 8,192 px a side and 128 MB at most. A file past them is a
//!   picture not drawn, not a crash. One that decodes to 2×2 or smaller is a
//!   tracking pixel and is not drawn either.
//! - **The web** (item 12) is fetched on a thread of its own, so a slow host
//!   never holds up filing: `https` or `http` only, no cookies, no
//!   `Referer`, 10 s, 5 MB a picture.
use std::io::Cursor;
use std::sync::mpsc::Sender;
use std::time::Duration;

use super::api::Gmail;
use crate::viewpane::bitmap::{self, Bitmap};

const WEB_BYTES: usize = 5 << 20;
const WEB_TIME: Duration = Duration::from_secs(10);

/// Bytes as a picture to draw, or `None`.
pub(crate) fn decode(bytes: &[u8]) -> Option<Bitmap> {
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(128 << 20);
    reader.limits(limits);
    let img = reader.decode().ok()?;
    let (w, h) = (img.width(), img.height());
    if w <= 2 && h <= 2 {
        return None;
    }
    bitmap::downscale(img, (w, h))
}

/// The worker's answer to [`Ask::Picture`]: a carried picture's bytes, from
/// inline `data` or attachment `att`, decoded under `key`.
///
/// [`Ask::Picture`]: super::worker::Ask::Picture
pub(crate) fn carried(
    g: &mut Gmail,
    key: String,
    msg: &str,
    att: Option<String>,
    data: Option<String>,
) -> super::worker::Got {
    let bytes = match (data.as_deref().and_then(super::body::bytes), att) {
        (Some(b), _) => Some(b),
        (None, Some(a)) => g.attachment(msg, &a).ok(),
        (None, None) => None,
    };
    super::worker::Got::Picture(key, bytes.and_then(|b| decode(&b)))
}

impl Gmail {
    /// An attachment's bytes (`messages.attachments.get`).
    pub(crate) fn attachment(&mut self, msg: &str, att: &str) -> Result<Vec<u8>, String> {
        let enc = super::query::enc;
        let v = self.get(&format!("/messages/{}/attachments/{}", enc(msg), enc(att)))?;
        v["data"]
            .as_str()
            .and_then(super::body::bytes)
            .ok_or_else(|| "Gmail sent an attachment with no bytes".into())
    }
}

/// Fetch `want` (`(key, address)`) from the web on a thread, sending each
/// picture (or `None`) back as it lands.
/// Pictures written into the mail (`html::inline`), decoded off the winit
/// thread, under the same limits as any other: nothing is fetched.
pub(crate) fn inline(want: Vec<(String, String)>, tx: Sender<(String, Option<Bitmap>)>) {
    std::thread::spawn(move || {
        for (k, payload) in want {
            let got = super::html::inline::bytes(&payload).and_then(|b| decode(&b));
            if tx.send((k, got)).is_err() {
                return; // the pane closed
            }
        }
    });
}

pub(crate) fn web(want: Vec<(String, String)>, tx: Sender<(String, Option<Bitmap>)>) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        let client = reqwest::Client::builder()
            .timeout(WEB_TIME)
            .referer(false)
            .redirect(reqwest::redirect::Policy::limited(5))
            .build();
        let (Ok(rt), Ok(client)) = (rt, client) else {
            for (k, _) in want {
                let _ = tx.send((k, None));
            }
            return;
        };
        rt.block_on(async {
            for (k, url) in want {
                let got = fetch(&client, &url).await.and_then(|b| decode(&b));
                if tx.send((k, got)).is_err() {
                    return; // the pane closed
                }
            }
        });
    });
}

async fn fetch(client: &reqwest::Client, url: &str) -> Option<Vec<u8>> {
    let lower = url.to_ascii_lowercase();
    if !(lower.starts_with("https://") || lower.starts_with("http://")) {
        return None;
    }
    let mut r = client.get(url).send().await.ok()?;
    let big = r.content_length().is_some_and(|n| n > WEB_BYTES as u64);
    if !r.status().is_success() || big {
        return None;
    }
    let mut out = Vec::new();
    while let Some(chunk) = r.chunk().await.ok()? {
        out.extend_from_slice(&chunk);
        if out.len() > WEB_BYTES {
            return None;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::decode;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(w, h, image::Rgba([200, 30, 30, 255]));
        let mut out = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }

    #[test]
    fn a_picture_decodes_and_a_pixel_or_junk_does_not() {
        assert!(decode(&png(40, 20)).is_some_and(|b| b.src == (40, 20)));
        assert!(decode(&png(1, 1)).is_none(), "a tracking pixel");
        assert!(decode(b"not a picture").is_none());
    }

    #[test]
    fn a_picture_past_the_limits_is_refused() {
        assert!(decode(&png(9000, 4)).is_none());
    }
}
