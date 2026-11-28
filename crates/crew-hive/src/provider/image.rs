//! A picture on its way to a model: the one type every provider renders, and
//! the checks that keep a bad file from failing the whole turn.
//!
//! Before this the wire carried text only: a pasted screenshot became a temp
//! path (`crew-app` `paste_image`) that CLI agents open themselves, while the
//! API engine sent the model the path STRING and got an answer from a model
//! that never saw the picture.
use std::sync::Arc;

/// Pictures one request carries from a task's text, and one tool round adds.
pub const MAX_IMAGES: usize = 5;

/// The most one picture may weigh ON THE WIRE (its base64 text). Anthropic's
/// docs allow 10 MB base64 on the Claude API but 5 MB on Amazon Bedrock and
/// Google-managed Vertex, so 5 MB is the cap that works on every route. Measured encoded rather than raw because that is the size
/// the API reads, so a file the API would refuse is refused here, with a note
/// the model can use, instead of failing the turn with a 400. (The docs say
/// base64-encoded.) See `imagecap` for the per-request limits.
pub const MAX_WIRE: usize = 5 * 1024 * 1024;

/// One image: its bytes already base64, because every request resends it and
/// encoding once per turn (not once per round) keeps earlier turns
/// byte-identical, which the prompt cache keys on. `Arc` so the clones a
/// request makes per round share one buffer.
#[derive(Clone, PartialEq)]
pub struct Image {
    /// The file name, for the model and for the notes.
    pub name: String,
    /// Sniffed from the bytes, never the extension: a `.png` that is a JPEG is
    /// a 400 from the API, which would fail the whole turn.
    pub media_type: &'static str,
    data: Arc<str>,
}

/// Megabytes of base64 must never reach a log or a failed-test dump.
impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Image({}, {}, {} B)",
            self.name,
            self.media_type,
            self.data.len()
        )
    }
}

impl Image {
    /// `bytes` as an image named `name`, or the reason it is not one the
    /// model can be given (a note, already worded for the model).
    pub fn from_bytes(name: &str, bytes: &[u8]) -> Result<Self, String> {
        let media_type =
            sniff(bytes).ok_or_else(|| omitted(name, "not a PNG, JPEG, GIF or WebP file"))?;
        if wire_len(bytes.len()) > MAX_WIRE {
            return Err(too_big(name, bytes.len()));
        }
        use base64::Engine;
        let data = base64::engine::general_purpose::STANDARD.encode(bytes);
        Ok(Self {
            name: name.to_string(),
            media_type,
            data: data.into(),
        })
    }

    /// Read `path` whole, or say why not. The size is checked before the read,
    /// so a 2 GB `.png` is never pulled into memory to be turned away.
    pub fn read(path: &std::path::Path) -> Result<Self, String> {
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into(),
        );
        let len = std::fs::metadata(path)
            .map_err(|e| omitted(&name, &e.to_string()))?
            .len() as usize;
        if wire_len(len) > MAX_WIRE {
            return Err(too_big(&name, len));
        }
        let bytes = std::fs::read(path).map_err(|e| omitted(&name, &e.to_string()))?;
        Self::from_bytes(&name, &bytes)
    }

    /// The base64 payload.
    pub fn data(&self) -> &str {
        &self.data
    }

    /// `data:<media type>;base64,<payload>`, the OpenAI wire's spelling.
    pub fn data_url(&self) -> String {
        format!("data:{};base64,{}", self.media_type, self.data)
    }
}

/// Base64 length of `n` bytes.
fn wire_len(n: usize) -> usize {
    n.div_ceil(3) * 4
}

fn too_big(name: &str, bytes: usize) -> String {
    omitted(
        name,
        &format!(
            "{:.1} MB is over the 5 MB limit for one image",
            bytes as f64 / 1_048_576.0
        ),
    )
}

/// `[image omitted: <name>, <why>]`: what the model is told in an image's
/// place, so it knows the picture existed and can say it could not see it.
pub fn omitted(name: &str, why: &str) -> String {
    format!("[image omitted: {name}, {why}]")
}

/// The note for a model that cannot read pictures.
pub fn text_only(name: &str) -> String {
    omitted(name, "this model reads text only")
}

/// The media type the first bytes say, or `None` for anything but a PNG,
/// JPEG, GIF or WebP (the four every vision API takes).
pub fn sniff(b: &[u8]) -> Option<&'static str> {
    match b {
        [0x89, b'P', b'N', b'G', ..] => Some("image/png"),
        [0xFF, 0xD8, 0xFF, ..] => Some("image/jpeg"),
        [b'G', b'I', b'F', b'8', ..] => Some("image/gif"),
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => Some("image/webp"),
        _ => None,
    }
}

/// Width and height when the header gives them cheaply (PNG, GIF, JPEG and
/// lossless WebP), else `None`: a label for the model, never worth decoding.
pub fn dimensions(b: &[u8]) -> Option<(u32, u32)> {
    let be = |at: usize| {
        b.get(at..at + 4)
            .map(|s| u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
    };
    let le16 = |at: usize| {
        b.get(at..at + 2)
            .map(|s| u32::from(u16::from_le_bytes([s[0], s[1]])))
    };
    let le24 = |at: usize| {
        b.get(at..at + 3)
            .map(|s| u32::from_le_bytes([s[0], s[1], s[2], 0]))
    };
    match sniff(b)? {
        "image/png" => Some((be(16)?, be(20)?)),
        "image/gif" => Some((le16(6)?, le16(8)?)),
        "image/webp" if b.get(12..16) == Some(b"VP8X") => Some((le24(24)? + 1, le24(27)? + 1)),
        "image/jpeg" => jpeg(b),
        _ => None,
    }
}

/// A JPEG's size is in its first start-of-frame segment; walk the segments.
fn jpeg(b: &[u8]) -> Option<(u32, u32)> {
    let mut at = 2;
    while at + 9 < b.len() {
        if b[at] != 0xFF {
            return None;
        }
        let marker = b[at + 1];
        let len = usize::from(u16::from_be_bytes([b[at + 2], b[at + 3]]));
        if matches!(marker, 0xC0..=0xCF) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            let h = u32::from(u16::from_be_bytes([b[at + 5], b[at + 6]]));
            let w = u32::from(u16::from_be_bytes([b[at + 7], b[at + 8]]));
            return Some((w, h));
        }
        at += 2 + len;
    }
    None
}

/// A PNG header of `w` x `h` and nothing else: enough to sniff and size.
#[cfg(test)]
pub(crate) fn fake_png(w: u32, h: u32) -> Vec<u8> {
    let mut b = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];
    b.extend_from_slice(b"IHDR");
    b.extend_from_slice(&w.to_be_bytes());
    b.extend_from_slice(&h.to_be_bytes());
    b
}

#[cfg(test)]
#[path = "image_tests.rs"]
mod tests;
