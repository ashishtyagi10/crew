//! The input bar's ghost text is what Tab would put there.
use super::InputBar;

fn ghost(text: &str, menu_sel: usize) -> Option<String> {
    InputBar {
        text: text.into(),
        focused: true,
        menu_sel,
        ..Default::default()
    }
    .ghost()
}

/// An open value picker ghosts its highlighted row: arrowed to `light`, the
/// bar read `/theme dark`, and Tab put in `light`.
#[test]
fn a_value_picker_ghosts_the_row_tab_fills() {
    assert_eq!(ghost("/theme ", 0).as_deref(), Some("dark"));
    assert_eq!(ghost("/theme ", 1).as_deref(), Some("light"));
    assert_eq!(ghost("/gradient ", 2).as_deref(), Some("lively"));
    // A command still ghosts the highlighted name's rest.
    assert_eq!(ghost("/s", 0).as_deref(), Some("ettings"));
    assert_eq!(ghost("/s", 1).as_deref(), Some("mith"));
}
