//! Which WAITING card row a sidebar row is — the one geometry the click
//! (`waiting_pane_at`) and the hover (`hovered_waiting_row`) read, so the
//! row that lifts under the pointer is the row a press focuses.
use crate::navlayout::NavLayout;

/// The card row at sidebar row `rel_row`, if `rel_row` is on the card's
/// body (past its border row and section rule).
pub(crate) fn at(rel_row: u16, l: &NavLayout) -> Option<usize> {
    if l.waiting_lines == 0 {
        return None;
    }
    // +1 for the border row, +1 to skip the section rule.
    let i = rel_row.checked_sub(l.waiting_top + 2)? as usize;
    (i < l.waiting_lines).then_some(i)
}

#[cfg(test)]
mod tests {
    use super::at;
    use crate::navlayout::NavLayout;

    #[test]
    fn only_the_body_rows_of_the_card_are_rows() {
        let l = NavLayout {
            waiting_top: 10,
            waiting_lines: 3,
            ..Default::default()
        };
        assert_eq!(at(11, &l), None, "the rule");
        assert_eq!(at(12, &l), Some(0));
        assert_eq!(at(14, &l), Some(2));
        assert_eq!(at(15, &l), None);
        assert_eq!(at(12, &NavLayout::default()), None, "no card");
    }
}
