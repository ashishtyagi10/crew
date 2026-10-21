//! `text-transform` from a mail's CSS (goal 2026-10-21-mail-looks-right,
//! round 5): the kicker over a headline (`THE ESSAY`), an issue line, a
//! button's label are set in capitals by CSS alone, and read as ordinary
//! words they lost the one thing that set them apart. Inherited, as in a
//! browser; applied to the words, never to an address.
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) enum Case {
    #[default]
    As,
    Upper,
    Lower,
    /// `capitalize`: each word's first letter.
    Title,
}

impl Case {
    /// The case `style` sets, or `None` where it sets none and the element
    /// keeps its parent's.
    pub(crate) fn of(style: &str) -> Option<Self> {
        match super::font::decl(style, "text-transform")?.as_str() {
            "uppercase" => Some(Self::Upper),
            "lowercase" => Some(Self::Lower),
            "capitalize" => Some(Self::Title),
            "none" => Some(Self::As),
            _ => None,
        }
    }

    pub(crate) fn apply(self, s: &str) -> Cow<'_, str> {
        match self {
            Self::As => Cow::Borrowed(s),
            Self::Upper => Cow::Owned(s.to_uppercase()),
            Self::Lower => Cow::Owned(s.to_lowercase()),
            Self::Title => {
                let mut start = true;
                let titled = s.chars().flat_map(|c| {
                    let up = start && c.is_alphabetic();
                    start = c.is_whitespace();
                    match up {
                        true => c.to_uppercase().collect::<Vec<_>>(),
                        false => vec![c],
                    }
                });
                Cow::Owned(titled.collect())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Case;

    #[test]
    fn a_case_is_read_off_the_style_and_set_on_the_words() {
        assert_eq!(
            Case::of("font-size:11px; text-transform: UPPERCASE"),
            Some(Case::Upper)
        );
        assert_eq!(Case::of("color:red"), None, "inherits");
        assert_eq!(Case::of("text-transform:none"), Some(Case::As));
        assert_eq!(Case::Upper.apply("The essay"), "THE ESSAY");
        assert_eq!(Case::Title.apply("shop the sale now"), "Shop The Sale Now");
        assert_eq!(Case::Lower.apply("SALE"), "sale");
        assert_eq!(Case::As.apply("As is"), "As is");
    }
}
