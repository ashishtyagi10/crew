//! The form's focusable elements and their Tab order. Split from `mod.rs`
//! for the 200-line cap when `auto`'s pairing pickers joined the Appearance
//! card.
//!
//! `FIELDS` is the Tab order and `Field` is the set; they are separate because
//! the declaration order groups related settings while the tab order follows
//! the eye down the cards. `every_config_key_is_editable_or_listed_as_absent`
//! holds this list against `CrewConfig` — a config key must be editable here
//! or listed as deliberately absent, which is how the `auto` settings came to
//! be here at all.

/// Focusable elements of the form, in Tab order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Field {
    FontFamily,
    FontSize,
    Smooth,
    NavWidth,
    ShowNav,
    Theme,
    /// `auto`'s per-appearance pairing: what it serves while the OS (or, when
    /// the appearance is pinned, the clock below) says dark / light.
    ThemeDark,
    ThemeLight,
    /// `auto`'s light-hours window, `HH:MM` each. Placed under Theme because
    /// they are only read while it is `auto` AND the OS appearance is pinned.
    LightFrom,
    LightTo,
    Accent,
    PaperTexture,
    PaperGrain,
    Glass,
    Motion,
    WindowOpacity,
    Maximized,
    Notify,
    NotifyAgentDone,
    NotifyBell,
    NotifyExit,
    NotifyMinSecs,
    NotifyPatterns,
    Save,
    Cancel,
}

pub(crate) const FIELDS: [Field; 25] = [
    Field::FontFamily,
    Field::FontSize,
    Field::Smooth,
    Field::NavWidth,
    Field::WindowOpacity,
    Field::ShowNav,
    Field::Theme,
    Field::ThemeDark,
    Field::ThemeLight,
    Field::LightFrom,
    Field::LightTo,
    Field::Accent,
    Field::PaperTexture,
    Field::PaperGrain,
    Field::Glass,
    Field::Motion,
    Field::Maximized,
    Field::Notify,
    Field::NotifyAgentDone,
    Field::NotifyBell,
    Field::NotifyExit,
    Field::NotifyMinSecs,
    Field::NotifyPatterns,
    Field::Save,
    Field::Cancel,
];
