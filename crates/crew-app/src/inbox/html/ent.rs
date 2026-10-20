//! HTML character references: `&amp;`, `&#8217;`, `&#x2019;`, and the
//! legacy forms mail still sends without the `;` (`&nbsp`).
//!
//! Not the whole HTML5 table (2,000+ names): the Latin-1 block, which is in
//! code-point order so one list covers 96 names, and the punctuation,
//! arrows and invisible joiners mail actually uses. An unknown name stays as
//! written, which is what a browser does too.

/// `&nbsp;` (U+00A0) to `&yuml;` (U+00FF), in order.
const LATIN1: &str = "nbsp iexcl cent pound curren yen brvbar sect uml copy ordf laquo not shy \
    reg macr deg plusmn sup2 sup3 acute micro para middot cedil sup1 ordm raquo frac14 frac12 \
    frac34 iquest Agrave Aacute Acirc Atilde Auml Aring AElig Ccedil Egrave Eacute Ecirc Euml \
    Igrave Iacute Icirc Iuml ETH Ntilde Ograve Oacute Ocirc Otilde Ouml times Oslash Ugrave \
    Uacute Ucirc Uuml Yacute THORN szlig agrave aacute acirc atilde auml aring aelig ccedil \
    egrave eacute ecirc euml igrave iacute icirc iuml eth ntilde ograve oacute ocirc otilde \
    ouml divide oslash ugrave uacute ucirc uuml yacute thorn yuml";

const OTHER: &[(&str, char)] = &[
    ("amp", '&'),
    ("lt", '<'),
    ("gt", '>'),
    ("quot", '"'),
    ("apos", '\''),
    ("ndash", '–'),
    ("mdash", '—'),
    ("lsquo", '‘'),
    ("rsquo", '’'),
    ("sbquo", '‚'),
    ("ldquo", '“'),
    ("rdquo", '”'),
    ("bdquo", '„'),
    ("dagger", '†'),
    ("Dagger", '‡'),
    ("bull", '•'),
    ("hellip", '…'),
    ("permil", '‰'),
    ("prime", '′'),
    ("lsaquo", '‹'),
    ("rsaquo", '›'),
    ("euro", '€'),
    ("trade", '™'),
    ("larr", '←'),
    ("uarr", '↑'),
    ("rarr", '→'),
    ("darr", '↓'),
    ("harr", '↔'),
    ("check", '✓'),
    ("hearts", '♥'),
    ("star", '☆'),
    ("OElig", 'Œ'),
    ("oelig", 'œ'),
    ("Scaron", 'Š'),
    ("scaron", 'š'),
    ("fnof", 'ƒ'),
    ("ensp", '\u{2002}'),
    ("emsp", '\u{2003}'),
    ("thinsp", '\u{2009}'),
    ("zwnj", '\u{200c}'),
    ("zwj", '\u{200d}'),
    ("lrm", '\u{200e}'),
    ("rlm", '\u{200f}'),
];

fn named(name: &str) -> Option<char> {
    if let Some(i) = LATIN1.split_whitespace().position(|n| n == name) {
        return char::from_u32(0xA0 + i as u32);
    }
    OTHER.iter().find(|(n, _)| *n == name).map(|&(_, c)| c)
}

fn numeric(digits: &str) -> Option<char> {
    let n = match digits.strip_prefix(['x', 'X']) {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => digits.parse().ok()?,
    };
    Some(match n {
        // HTML reads these as windows-1252, as mailers mean them.
        0x80..=0x9F => crate::inbox::charset::cp1252(n as u8),
        0 => '\u{fffd}',
        _ => char::from_u32(n).unwrap_or('\u{fffd}'),
    })
}

/// `text` with every reference it holds replaced.
pub(crate) fn decode(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at + 1..];
        let end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '#'))
            .unwrap_or(rest.len());
        let (word, semi) = (&rest[..end], rest[end..].starts_with(';'));
        let got = match word.strip_prefix('#') {
            Some(d) => numeric(d),
            // Without the `;` only the legacy names count, as in a browser.
            None if semi || named(word).is_some_and(|c| c.is_ascii() || c == '\u{a0}') => {
                named(word)
            }
            None => None,
        };
        match got {
            Some(c) => {
                out.push(c);
                rest = &rest[end + usize::from(semi)..];
            }
            None => out.push('&'),
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn named_numeric_and_legacy_references_decode() {
        assert_eq!(
            decode("Tom&nbsp;&amp; Jerry&#8217;s caf&eacute; &#x2014; &#150;"),
            "Tom\u{a0}& Jerry\u{2019}s caf\u{e9} \u{2014} \u{2013}"
        );
        assert_eq!(decode("a&nbsp b &amp c"), "a\u{a0} b & c");
        assert_eq!(decode("AT&T &unknown; &"), "AT&T &unknown; &");
    }
}
