//! ↑ / ↓ on a number field steps it, the way a native number input or a
//! macOS stepper does. Typing a number is still the way to say an exact
//! one; the arrows are for "a bit more, a bit less", which is how opacity,
//! grain and font size actually get tuned — a keystroke per notch with the
//! value in view, instead of backspacing and retyping.
//!
//! The step lands in the edit buffer and is committed at once, so the value
//! is clamped by the SAME rule a typed one is (`commit::commit_field`) —
//! stepping past a bound stops at the bound rather than inventing a second
//! range to keep in sync.
use super::commit::commit_field;
use super::keys::buf_of;
use super::{Field, SettingsPane};

/// One notch for `f`, or `None` for a field that is not a stepped number.
/// Sized so a notch is visible and ten of them cross most of the range.
pub(super) fn notch(f: Field) -> Option<f64> {
    Some(match f {
        Field::FontSize => 1.0,
        Field::NavWidth => 10.0,
        Field::PaperGrain => 0.1,
        Field::WindowOpacity => 5.0,
        Field::NotifyMinSecs => 1.0,
        Field::Budget5h | Field::Budget7d => 1.0,
        _ => return None,
    })
}

/// Step the focused field one notch up (or down); `big` takes ten. Returns
/// whether the field was a stepped number at all.
pub(super) fn step(p: &mut SettingsPane, up: bool, big: bool) -> bool {
    let f = p.focused_field();
    let Some(n) = notch(f) else {
        return false;
    };
    // Settle a half-typed buffer first, so the step is from the value the
    // field would commit to, not from whatever text is mid-edit.
    commit_field(p);
    let Some(buf) = buf_of(p, f) else {
        return false;
    };
    let Ok(now) = buf.trim().parse::<f64>() else {
        return true;
    };
    let delta = n * if big { 10.0 } else { 1.0 } * if up { 1.0 } else { -1.0 };
    // Round to the notch's own precision: 1.3 + 0.1 is 1.4000000000000001.
    let next = ((now + delta) / n).round() * n;
    *buf = if n < 1.0 {
        format!("{next:.1}")
    } else {
        format!("{}", next.max(0.0) as i64)
    };
    commit_field(p);
    true
}

#[cfg(test)]
#[path = "stepper_tests.rs"]
mod tests;
