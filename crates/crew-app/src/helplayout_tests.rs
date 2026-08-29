use super::*;

/// `{left:<26}` pads to a minimum; it does not guarantee a gap. Five bindings
/// are wider than 26 columns, and their descriptions ran straight into the
/// keys — `Cmd+wheelFont size + / - / reset`. Nothing may touch.
#[test]
fn no_key_ever_touches_its_description() {
    let cols = 100u16;
    let col = key_col(cols);
    for (k, _) in logical().iter().filter(|(k, _)| !k.is_empty()) {
        let pad = col.saturating_sub(str_w(k)).max(2);
        assert!(pad >= 2, "{k:?} left no gap before its description");
    }
    assert!(
        col >= widest_key() + 2,
        "a wide panel aligns every key: col {col} vs widest {}",
        widest_key()
    );
}

/// The description is the half that teaches, so the keys never take more than
/// 45% of a narrow panel — they go ragged instead.
#[test]
fn the_key_column_cannot_swallow_a_narrow_panel() {
    for cols in [40u16, 50, 60, 80] {
        assert!(
            key_col(cols) * 100 <= cols as usize * 45,
            "{cols}: key column {} took more than 45%",
            key_col(cols)
        );
    }
}

/// ratatui clips a `Line` in silence. At any window narrower than the panel
/// asks for, half the descriptions read as sentence fragments. They wrap now.
#[test]
fn a_description_too_wide_to_fit_wraps_instead_of_vanishing() {
    let narrow = 62u16;
    let laid = rows("", narrow);
    let text: Vec<String> = laid
        .iter()
        .filter_map(|r| match r {
            Row::Bind(_, d) | Row::Cont(d) => Some(d.clone()),
            _ => None,
        })
        .collect();
    let joined = text.join(" ");
    // The binding whose tail used to be cut off ("...or /find in the ba").
    assert!(
        joined.contains("/find in the bar"),
        "the tail of a long description survived the narrow panel"
    );
    // Nothing on any line exceeds the room the description column has.
    let width = (narrow as usize) - 2 - key_col(narrow);
    for d in &text {
        assert!(str_w(d) <= width, "{d:?} is wider than {width}");
    }
    // And it cost rows, which is what scrolling is for.
    assert!(
        laid.len() > logical().len(),
        "wrapping produced no extra display lines"
    );
}

/// A panel wide enough for everything wraps nothing.
#[test]
fn the_preferred_width_wraps_nothing() {
    let laid = rows("", crate::help::size().0);
    assert!(
        !laid.iter().any(|r| matches!(r, Row::Cont(_))),
        "the preferred width had to wrap"
    );
    assert_eq!(laid.len(), logical().len());
}

/// A search that matches nothing says so rather than showing a blank panel.
#[test]
fn a_search_with_no_hits_says_so() {
    let laid = rows("zzzznope", 100);
    assert!(matches!(laid.as_slice(), [Row::Note(_)]));
}

/// A heading survives only when a row under it did.
#[test]
fn a_filtered_list_keeps_only_the_headings_it_needs() {
    let laid = rows("reverse-search", 100);
    let heads: Vec<&str> = laid
        .iter()
        .filter_map(|r| match r {
            Row::Head(h) => Some(*h),
            _ => None,
        })
        .collect();
    assert_eq!(heads, vec!["in an agent pane"]);
}
