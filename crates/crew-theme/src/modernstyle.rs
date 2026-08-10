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
}
