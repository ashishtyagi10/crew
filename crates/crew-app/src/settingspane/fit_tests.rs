use super::*;

/// The case this module exists for: `sepia-light` in a half-width box at
/// an 80-column pane. The number here is the one the old comment
/// recorded, and it must still come out too wide to pair.
#[test]
fn a_palette_picker_does_not_fit_a_half_box_at_eighty_columns() {
    // 80-col pane → card inner width 76 → half is 37. A palette picker
    // needs more than a half of the inner width the ORIGINAL bug had:
    // the card was narrower than the pane. Assert on the cost directly.
    let need = min_cols(Field::ThemeDark);
    assert!(
        need > 14,
        "a palette picker fitting 14 columns is the old bug"
    );
    // …and it is the VALUE that makes it wide, not the legend.
    assert!(
        need > (super::super::labels::label_of(Field::ThemeDark)
            .chars()
            .count()
            + CHROME) as u16,
        "the widest value has to be what drives the cost"
    );
}

/// A cost that does not cover the legend gives a truncated label, which is
/// the same defect one row up.
#[test]
fn every_field_can_at_least_carry_its_own_legend() {
    for f in super::super::fields::FIELDS {
        let legend = super::super::labels::label_of(f).chars().count();
        assert!(
            min_cols(f) as usize >= legend + CHROME,
            "{f:?} costs {} but its legend is {legend}",
            min_cols(f)
        );
    }
}

/// The picker costs come from the same lists the cycler steps through, so
/// a new option cannot quietly outgrow its box. Spot-check that the widest
/// option of each closed set is actually covered.
#[test]
fn a_pickers_cost_covers_its_own_widest_option() {
    for (f, longest) in [
        (Field::Density, "compact".len()),
        (Field::Leading, "relaxed".len()),
        (Field::Contrast, "normal".len()),
        (Field::Glass, "medium".len()),
    ] {
        assert!(
            min_cols(f) as usize >= longest + CHROME,
            "{f:?} cannot carry its widest option"
        );
    }
}

/// A picker's widest value fits its box whole, chevrons, their spaces and
/// all (`‹ auto (full) ›`): the box gives a value its width less 4, and the
/// spaces inside the chevrons were not counted, so a 17-column Motion box
/// kept the end of its 15-column value and lost the `‹ `.
#[test]
fn a_pickers_widest_value_fits_with_its_chevrons() {
    for f in [
        Field::Motion,
        Field::Theme,
        Field::ThemeDark,
        Field::ThemeLight,
    ] {
        let widest = widest_value(f).expect("a picker");
        let shown = format!("\u{2039} {} \u{203a}", "x".repeat(widest));
        let room = usize::from(min_cols(f)) - 4;
        assert!(
            room >= shown.chars().count(),
            "{f:?}: {room} < {}",
            shown.chars().count()
        );
    }
}
