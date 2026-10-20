//! A mail part's bytes as text, by the `charset` its `Content-Type` names.
//!
//! UTF-8 and ASCII are what most mail is. Latin-1 and windows-1252 are what
//! most of the rest is, and they are one table: browsers read `iso-8859-1`
//! as windows-1252 (the WHATWG encoding standard), because mailers label
//! their curly quotes Latin-1 when they are really 0x91–0x94. Anything else
//! keeps the old lossy UTF-8 reading and says so, rather than pulling in a
//! decoder for every charset there is (goal 2026-10-20-mail-reader, item 8).

/// windows-1252's 0x80–0x9F, the only bytes where it differs from Latin-1.
/// `\0` marks the five it leaves undefined, which stay as their code point.
const HIGH: [char; 32] = [
    '€', '\0', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\0', 'Ž', '\0', '\0', '‘',
    '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\0', 'ž', 'Ÿ',
];

/// The `charset` parameter of a `Content-Type` value, lowercased.
pub(crate) fn of(content_type: &str) -> Option<String> {
    content_type.split(';').skip(1).find_map(|p| {
        let (k, v) = p.split_once('=')?;
        (k.trim().eq_ignore_ascii_case("charset"))
            .then(|| v.trim().trim_matches(['"', '\'']).to_ascii_lowercase())
    })
}

/// `bytes` as text, and the charset's name when it was not one crew reads
/// (the text is then a lossy UTF-8 guess).
pub(crate) fn decode(bytes: &[u8], charset: Option<&str>) -> (String, Option<String>) {
    let cs = charset.unwrap_or("utf-8").trim().to_ascii_lowercase();
    match cs.as_str() {
        "" | "utf-8" | "utf8" | "us-ascii" | "ascii" => {
            (String::from_utf8_lossy(bytes).into_owned(), None)
        }
        "iso-8859-1" | "iso8859-1" | "latin1" | "latin-1" | "l1" | "windows-1252" | "cp1252"
        | "x-cp1252" => (bytes.iter().map(|&b| cp1252(b)).collect(), None),
        _ => (String::from_utf8_lossy(bytes).into_owned(), Some(cs)),
    }
}

/// One windows-1252 byte (also what HTML says `&#150;` means).
pub(crate) fn cp1252(b: u8) -> char {
    match b {
        0x80..=0x9F => match HIGH[usize::from(b - 0x80)] {
            '\0' => char::from(b),
            c => c,
        },
        _ => char::from(b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_1252_quotes_and_latin_1_letters_read_right() {
        let bytes = [0x93, b'h', 0xE9, 0x94, b' ', 0x96, b' ', 0x80, b'5'];
        assert_eq!(
            decode(&bytes, Some("windows-1252")),
            ("\u{201c}h\u{e9}\u{201d} \u{2013} \u{20ac}5".into(), None)
        );
        assert_eq!(
            decode(&bytes, Some("ISO-8859-1")).0,
            decode(&bytes, Some("cp1252")).0
        );
    }

    #[test]
    fn utf_8_is_the_default_and_an_unknown_charset_is_named() {
        assert_eq!(decode("é".as_bytes(), None), ("é".into(), None));
        assert_eq!(
            decode(b"abc", Some("Shift_JIS")),
            ("abc".into(), Some("shift_jis".into()))
        );
    }

    #[test]
    fn the_charset_is_read_off_a_content_type() {
        assert_eq!(
            of("text/html; charset=\"Windows-1252\"; format=flowed").as_deref(),
            Some("windows-1252")
        );
        assert_eq!(of("text/plain"), None);
    }
}
