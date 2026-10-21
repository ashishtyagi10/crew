//! A picture the mail writes into itself (`data:image/png;base64,…`, goal
//! 2026-10-21-mail-looks-right, item 5): kept beside the markdown under a
//! short name made from its bytes, so the reader decodes it in place with
//! nothing fetched, under the limits every picture has. The markdown never
//! holds the payload: an address longer than 2 KB is no picture at all.
use std::hash::{Hash, Hasher};

use super::out::Out;

/// The most of a payload kept: 5 MB of picture, written in base64.
const MOST: usize = 5 * 1024 * 1024 * 4 / 3;

/// The kinds of picture decoded; anything else (an SVG, which can script)
/// stays its words.
const KINDS: &[&str] = &["png", "jpeg", "jpg", "gif", "webp"];

/// The name `src` is drawn under, when it is a picture to decode: kept, with
/// its payload, in `o.inline`.
pub(crate) fn keep(src: &str, o: &mut Out) -> Option<String> {
    let rest = src
        .get(..11)?
        .eq_ignore_ascii_case("data:image/")
        .then(|| &src[11..])?;
    let (head, payload) = rest.split_once(',')?;
    let mut parts = head.split(';');
    let kind = parts.next()?.to_ascii_lowercase();
    if !KINDS.contains(&kind.as_str()) || !parts.any(|p| p.eq_ignore_ascii_case("base64")) {
        return None;
    }
    if payload.len() > MOST || payload.is_empty() {
        return None;
    }
    let mut h = std::hash::DefaultHasher::new();
    payload.hash(&mut h);
    let key = format!("data:{:016x}", h.finish());
    if !o.inline.iter().any(|(k, _)| *k == key) {
        o.inline.push((key.clone(), payload.to_string()));
    }
    Some(key)
}

/// A payload's bytes: base64, as a mail writes it (any line breaks out).
pub(crate) fn bytes(payload: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    let clean: String = payload.chars().filter(|c| !c.is_whitespace()).collect();
    base64::engine::general_purpose::STANDARD_NO_PAD
        .decode(clean.trim_end_matches('='))
        .ok()
}

#[cfg(test)]
mod tests {
    use super::super::out::Out;

    #[test]
    fn a_picture_written_into_the_mail_is_kept_by_name_and_an_svg_is_not() {
        let mut o = Out::default();
        let png = "data:image/png;base64,iVBORw0KGgo=";
        let key = super::keep(png, &mut o).expect("a png");
        assert!(key.starts_with("data:") && key.len() == 21, "{key}");
        assert_eq!(super::keep(png, &mut o), Some(key), "once, by its bytes");
        assert_eq!(o.inline.len(), 1);
        assert_eq!(
            super::keep("data:image/svg+xml;base64,PHN2Zz4=", &mut o),
            None
        );
        assert_eq!(
            super::keep("data:image/png,raw", &mut o),
            None,
            "not base64"
        );
        assert_eq!(
            super::bytes("iVBO\nRw0K").as_deref(),
            Some(&[137, 80, 78, 71, 13, 10][..])
        );
    }
}
