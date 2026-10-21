//! A mail's button drawn as a button (the reader loop, round 4: "how action
//! button appear"): the converter writes one as a link around a padded code
//! span (`` [`  Shop now  `](…) ``, `inbox::html::anchor`), and it was drawn
//! as inline code is, a grey chip with link-blue words, quieter than the
//! call to action the mail meant. Now it is filled with the link's colour,
//! its words in whichever of the page and the ink reads hardest on it.
use crate::chatbody::Color;
use crate::md::MdSpan;

/// The words' ink and the fill for `span`, when it is a button.
pub(crate) fn ink(span: &MdSpan) -> Option<(Color, Color)> {
    let t = &span.text;
    let padded = t.chars().count() > 2 && t.starts_with(' ') && t.ends_with(' ');
    if !(span.style.code && span.link.is_some() && padded) {
        return None;
    }
    let fill = crate::chatink::link_color();
    let th = crew_theme::theme();
    let read = |c: Color| crew_theme::contrast_ratio(c, fill);
    let words = match read(th.page_bg) >= read(th.ink) {
        true => th.page_bg,
        false => th.ink,
    };
    Some((words, fill))
}

#[cfg(test)]
mod tests {
    use crate::md::MdSpan;

    #[test]
    fn a_padded_code_link_is_a_filled_button_and_inline_code_is_not() {
        let _g = crate::app::theme_test_guard();
        let span = |text: &str, link: bool| MdSpan {
            text: text.into(),
            style: crate::md::MdStyle {
                code: true,
                ..Default::default()
            },
            link: link.then(|| "https://x.io".to_string()),
            src: None,
        };
        let (words, fill) = super::ink(&span(" Shop now ", true)).expect("a button");
        assert_eq!(fill, crate::chatink::link_color());
        assert!(
            crew_theme::contrast_ratio(words, fill) >= 3.0,
            "{words:?} on {fill:?}"
        );
        assert!(
            super::ink(&span("fn main", true)).is_none(),
            "inline code in a link"
        );
        assert!(super::ink(&span(" Shop now ", false)).is_none(), "no link");
    }
}
