use super::*;

/// DECSCUSR is how a program says what mode it is in — a bar for insert, a
/// block for normal. Drawing every one of them as a block loses that.
#[test]
fn the_focused_pane_draws_the_shape_the_program_asked_for() {
    for (asked, want) in [
        (TermShape::Block, CursorShape::Block),
        (TermShape::Beam, CursorShape::Beam),
        (TermShape::Underline, CursorShape::Underline),
        (TermShape::HollowBlock, CursorShape::Hollow),
    ] {
        assert_eq!(shape_for(asked, true), want, "{asked:?}");
    }
}

/// Whichever shape the program asked for, a pane that is not taking keys shows
/// an outline — so however many panes are open, exactly one filled cursor is.
#[test]
fn an_unfocused_pane_outlines_whatever_shape_it_is_in() {
    for asked in [TermShape::Block, TermShape::Beam, TermShape::Underline] {
        assert_eq!(shape_for(asked, false), CursorShape::Hollow, "{asked:?}");
    }
}

/// A hidden cursor stays hidden in both panes — an unfocused pane must not
/// resurrect it as an outline.
#[test]
fn a_hidden_cursor_draws_nothing_in_either_pane() {
    assert_eq!(shape_for(TermShape::Hidden, true), CursorShape::None);
    assert_eq!(shape_for(TermShape::Hidden, false), CursorShape::None);
}
