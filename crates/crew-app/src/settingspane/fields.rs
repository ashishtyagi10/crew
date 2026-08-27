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
    /// Whether pane cards mark what they know on their borders — the ticks
    /// where a command began and the bars beside error lines.
    BorderMarks,
    PaperTexture,
    /// Whether the page's gradient wash keeps drifting while nothing is
    /// happening (see `washphase`). Sits beside Paper texture: both decide
    /// what the page itself does behind the cards.
    AmbientDrift,
    PaperGrain,
    Glass,
    Motion,
    /// How tightly the canvas packs — the pane gutter and the rows between
    /// chat cards (see `density`). Sits under Glass/Motion: those say how much
    /// the canvas *does*, this says how much room it takes.
    Density,
    /// How much air sits between rows of text — the cell height as a
    /// fraction of the font size (see `leading`). Beside Density because the
    /// two are the pair people confuse: density is how much crew fits on the
    /// canvas, leading is how the text reads.
    Leading,
    /// WCAG floor every derived colour is measured against: `auto` follows
    /// the OS accessibility switch (see `crew_theme::contrast`).
    Contrast,
    /// Whether meaning is said with a shape as well as a colour (WCAG 1.4.1);
    /// `auto` follows the OS switch. See `shapecues`.
    ShapeCues,
    /// How far the gradient's colour leans from the theme's own over time
    /// (see `gradientlvl`). Sits with Glass and Motion: all three say how
    /// much the canvas is allowed to do on its own.
    Gradient,
    WindowOpacity,
    Maximized,
    Notify,
    NotifyAgentDone,
    NotifyBell,
    NotifyExit,
    NotifyMinSecs,
    NotifyPatterns,
    /// Token budgets the footer's 5h / 7d bars are drawn against. Typed in
    /// millions — see `tokens`.
    Budget5h,
    Budget7d,
    Save,
    Cancel,
}

pub(crate) const FIELDS: [Field; 34] = [
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
    Field::BorderMarks,
    Field::PaperTexture,
    Field::AmbientDrift,
    Field::PaperGrain,
    Field::Glass,
    Field::Motion,
    Field::Density,
    Field::Leading,
    Field::Contrast,
    Field::ShapeCues,
    Field::Gradient,
    Field::Maximized,
    Field::Notify,
    Field::NotifyAgentDone,
    Field::NotifyBell,
    Field::NotifyExit,
    Field::NotifyMinSecs,
    Field::NotifyPatterns,
    Field::Budget5h,
    Field::Budget7d,
    Field::Save,
    Field::Cancel,
];
