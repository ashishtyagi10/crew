//! How many chips a list's swatch column may take on a narrow card.
//!
//! The column is the list's widest swatch so every description starts in
//! one place (`cmdrow::swatch_col`). On `/theme` the widest is `auto`'s
//! strip of every look, fifteen chips: at a quarter tile that column left
//! each description seven letters (`black…`, `rotatin…`), on every row,
//! including the ones with a single chip. A strip cut to a few chips still
//! reads as "many colours"; a description cut to one word reads as nothing.

/// Columns of description a row keeps before its swatch is cut: two or
/// three words (`black page, white…`), and at a quarter tile still room for
/// a palette's six chips whole — only `auto`'s strip is cut.
const DESC_WANT: usize = 16;

/// The swatch column for `items` laid out in `avail` columns with a label
/// column `label_w` wide: the widest swatch, cut to what leaves the
/// description `DESC_WANT` columns. Never under one chip while any row has a
/// swatch — on a row whose subject is a colour, one chip still says it.
pub(crate) fn col(items: &[crate::suggest::MenuItem], label_w: usize, avail: usize) -> usize {
    let widest = crate::cmdrow::swatch_col(items);
    if widest == 0 {
        return 0;
    }
    let gaps = 2 * crate::cmdrow::GAP;
    let cap = avail.saturating_sub(label_w + gaps + DESC_WANT).max(1);
    widest.min(cap)
}

#[cfg(test)]
#[path = "swatchfit_tests.rs"]
mod tests;
