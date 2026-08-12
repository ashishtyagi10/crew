//! Per-theme tuning for the MODERN family (goal 2026-08-10): the Gemini /
//! Codex-app look — deep neutral pages, vibrant accents, soft wide glow and a
//! gradient light-ring on the focused frame instead of a single-colour
//! stroke. Like `CrtStyle` this is pure data: crew-theme states the two
//! gradient poles and the drift period, crew-app's ring painter and the
//! renderer's bloom chain do the drawing. A modern theme carries BOTH styles
//! — its `CrtStyle` runs the bloom with every retro knob (curvature,
//! scanlines, bezel) at zero, so the halo is clean light, not a tube.

/// The modern-family knobs a theme ships.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModernStyle {
    /// The two colours the focused frame's gradient ring runs between. The
    /// ring blends them around the perimeter with a seamless cosine loop, so
    /// `pole_a` and `pole_b` each appear twice, on opposite sides.
    pub pole_a: (u8, u8, u8),
    pub pole_b: (u8, u8, u8),
    /// Period of one full gradient revolution while the pane is streaming,
    /// in ms. Idle frames never drift (the static-frame determinism
    /// contract) — this only paces the motion during activity.
    pub drift_ms: u64,
    /// Strength of the page's dot lattice (0 = none): a faint grid of soft
    /// dots behind everything, tinted `pole_a`→`pole_b` across the page
    /// diagonal — the modern family's engineering-paper backdrop. It is a
    /// mix weight toward the pole colour, so ~0.2 reads as a whisper on a
    /// dark page. Static: the lattice never animates or requests frames.
    pub dots: f32,
    /// Strength of the page's gradient wash (0 = none): two broad soft pools
    /// of `pole_a` and `pole_b` light lying under the whole page, the aurora
    /// behind the engineering paper. A mix weight toward the pole colour at
    /// each pool's centre, falling to zero between them — ~0.15 lifts a deep
    /// page just enough to read as coloured light rather than a flat fill.
    ///
    /// The two pools sit on opposite sides of the page and rotate about its
    /// centre, one revolution per `drift_ms`, but ONLY while a pane is busy:
    /// the phase is advanced by the app (see crew-app's `washphase`) from the
    /// frames activity is already drawing, and holds wherever it stopped once
    /// things go quiet. An idle crew still never repaints.
    pub wash: f32,
}
