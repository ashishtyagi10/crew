//! A picture the mail gave no height, sized by the picture itself once it
//! lands (goal 2026-10-21-mail-looks-right, item 5): a hero takes the
//! measure and a logo stays a logo. Until it lands its box is the engine's
//! own, and the words under it move once, when it does, as they do in Gmail.
use std::borrow::Cow;
use std::collections::HashMap;

use super::InboxPane;

/// A mail's px in a cell: about 8 across and 16 down.
const COL_PX: f32 = 8.0;
const ROW_PX: f32 = 16.0;

/// The size each landed picture really is, by its key (`pics::key`).
pub(crate) type Sizes = HashMap<String, (u32, u32)>;

/// `md` with each `#rows=N&fit=W` whose picture has landed given the rows
/// its shape takes at `width` columns: as wide as the mail asks (`W` px, or
/// the picture's own width when it asks nothing), never wider than the text.
pub(crate) fn fitted<'a>(md: &'a str, msg: &str, width: usize, sizes: &Sizes) -> Cow<'a, str> {
    if sizes.is_empty() || !md.contains("&fit=") {
        return Cow::Borrowed(md);
    }
    let mut out = String::with_capacity(md.len());
    let mut rest = md;
    while let Some(at) = rest.find("#rows=") {
        let (head, tail) = rest.split_at(at);
        out.push_str(head);
        let src = head.rsplit("](<").next().unwrap_or("");
        let end = tail.find(">)").unwrap_or(tail.len());
        let (frag, after) = tail.split_at(end);
        let fit = frag
            .split_once("&fit=")
            .and_then(|(_, w)| w.parse::<f32>().ok());
        let size = sizes
            .get(&super::pics::key(msg, src))
            .filter(|(w, h)| *w > 0 && *h > 0);
        match (fit, size) {
            (Some(asked), Some(&(w, h))) => {
                let measure = width as f32 * COL_PX;
                let shown = if asked > 0.0 { asked } else { w as f32 }.min(measure);
                let rows = (shown * h as f32 / w as f32 / ROW_PX)
                    .round()
                    .clamp(2.0, 40.0);
                out.push_str(&format!("#rows={rows}"));
            }
            _ => out.push_str(frag),
        }
        rest = after;
    }
    out.push_str(rest);
    Cow::Owned(out)
}

impl InboxPane {
    /// The landed pictures' sizes, handed to the open thread's layout.
    pub(crate) fn sync_sizes(&mut self) {
        if let Some(r) = self.reading.as_mut() {
            r.sizes = self.pics.sizes();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{fitted, Sizes};

    #[test]
    fn a_landed_picture_takes_the_rows_its_shape_needs_and_never_more_than_the_measure() {
        let md = "![hero](<https://x.io/h.png#rows=12&fit=600>)\n\n![logo](<https://x.io/l.png#rows=12&fit=0>)\n\n![set](<https://x.io/s.png#rows=4>)";
        let sizes: Sizes = [
            ("https://x.io/h.png".to_string(), (1200, 600)),
            ("https://x.io/l.png".to_string(), (160, 48)),
            ("https://x.io/s.png".to_string(), (100, 100)),
        ]
        .into_iter()
        .collect();
        let got = fitted(md, "m1", 80, &sizes);
        assert!(got.contains("h.png#rows=19>)"), "600 px wide, 2:1: {got}");
        assert!(got.contains("l.png#rows=3>)"), "its own 160 px: {got}");
        assert!(
            got.contains("s.png#rows=4>)"),
            "a height the mail gave stays: {got}"
        );
        let narrow = fitted(md, "m1", 40, &sizes);
        assert!(
            narrow.contains("h.png#rows=10>)"),
            "the measure caps it: {narrow}"
        );
        let none = fitted(md, "m1", 80, &Sizes::new());
        assert_eq!(none, md, "nothing landed: the engine's own box");
    }
}
