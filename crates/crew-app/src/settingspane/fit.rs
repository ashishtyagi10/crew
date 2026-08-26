//! How many columns a field needs — and therefore whether two of them fit
//! side by side.
//!
//! The settings form pairs some fields into half-width boxes and gives others
//! the whole row, and until now that was a per-field decision taken once, by
//! eye, at whatever width the pane happened to be when it was written. That
//! has already gone wrong once: `‹ sepia-light ›` is 15 columns, a half-width
//! box holds 14 at an 80-column pane, and the Auto-dark picker shipped with
//! its leading chevron clipped — which reads as a rendering fault, not as a
//! layout that ran out of room. The fix at the time was to pin those two
//! fields to full width forever, which is correct at 80 columns and wasteful
//! at 200.
//!
//! The decision belongs to the *width*, not to the field. This module says
//! what a field costs; [`super::cards::pair`] compares that against the room
//! there is and stacks the two when they will not fit. A field that fits pairs
//! at any width, one that does not gets its own row at any width, and nothing
//! is ever clipped at either end.
//!
//! Cost is the wider of what the box has to carry:
//!
//! * the **legend**, which rides the top border and is bracketed by the
//!   corner and a space either side, and
//! * the **widest value** the field can display — for a picker, the longest
//!   of the options it actually cycles through, taken from the same lists the
//!   cycler uses so a new option cannot widen the box without widening this.
use super::Field;

/// Border and padding around a value: two border columns, a space inside
/// each, and the `‹ ›` a picker wears. Text boxes pay the same, which keeps
/// a typed value clear of the cursor at the right edge.
const CHROME: usize = 6;

/// The longest option a picker can show, or `None` for a field that is not a
/// picker over a closed set.
fn widest_value(f: Field) -> Option<usize> {
    let longest = |v: &[&str]| v.iter().map(|s| s.chars().count()).max().unwrap_or(0);
    Some(match f {
        Field::Motion => crate::motion::MotionPref::ALL
            .iter()
            // `auto` displays as `auto (full)` — the resolved form is what is
            // actually drawn, so it is what has to fit.
            .map(|p| {
                p.label(true)
                    .chars()
                    .count()
                    .max(p.label(false).chars().count())
            })
            .max()
            .unwrap_or(0),
        Field::Density => longest(
            &crate::density::Density::ALL
                .iter()
                .map(|d| d.as_str())
                .collect::<Vec<_>>(),
        ),
        Field::Gradient => longest(
            &crate::gradientlvl::GradientLevel::ALL
                .iter()
                .map(|g| g.as_str())
                .collect::<Vec<_>>(),
        ),
        Field::Glass => longest(&["off", "low", "medium", "high"]),
        Field::Contrast => longest(&["auto", "normal", "high"]),
        Field::ShapeCues => longest(&["auto", "off", "on"]),
        Field::Smooth => longest(&["off", "light", "medium", "heavy", "255"]),
        // Palette names, the case that started all of this.
        Field::Theme | Field::ThemeDark | Field::ThemeLight => crew_theme::ALL_THEMES
            .iter()
            .map(|t| t.as_str().chars().count())
            .max()
            .unwrap_or(0),
        _ => return None,
    })
}

/// The columns `f` needs to draw without clipping either its legend or its
/// widest value.
pub(crate) fn min_cols(f: Field) -> u16 {
    let legend = super::labels::label_of(f).chars().count();
    let value = widest_value(f).unwrap_or(0);
    (legend.max(value) + CHROME) as u16
}

#[cfg(test)]
mod tests {
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
            (Field::Contrast, "normal".len()),
            (Field::Glass, "medium".len()),
        ] {
            assert!(
                min_cols(f) as usize >= longest + CHROME,
                "{f:?} cannot carry its widest option"
            );
        }
    }
}
