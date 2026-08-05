//! Per-theme CRT tube tuning. The post-process knobs used to be four global
//! constants in crew-render's `crt.rs`, which meant every phosphor rendered
//! with the same personality; now each `CRT_*` preset carries its own
//! `CrtStyle` (`Theme.crt: Option<CrtStyle>`) so a hot P1 green and a cold
//! TRON blue can actually differ. crew-theme stays data-only — the renderer
//! reads these numbers into its uniforms, nothing here touches the GPU.

/// The CRT post-process knobs a theme ships. All amounts are in the shader's
/// working space (the surface format's encoded values — see crt.wgsl).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrtStyle {
    /// Barrel-warp strength; 0 keeps the panel flat and edge-to-edge.
    pub curvature: f32,
    /// Scanline darkening weight (0 = no raster lines).
    pub scanline: f32,
    /// Bloom composite strength: the blurred bright-pass is added back in
    /// scaled by this, so it is the "how much does light bleed" knob.
    pub glow: f32,
    /// Bloom blur radius in HALF-RES pixels — the blur chain runs on a
    /// half-resolution target, so the full-res reach is roughly 2× this.
    pub glow_radius: f32,
    /// Corner/tube-face vignette depth; 0 = no bezel falloff.
    pub corner: f32,
    /// Brightness-wobble amplitude while a pane is streaming. Idle frames
    /// always run at 0 regardless (the static-tube determinism contract);
    /// this is only what the app dials in during activity.
    pub flicker: f32,
}

impl CrtStyle {
    /// Today's look before the per-theme split: a flat phosphor panel
    /// (no warp, no bezel) with moderate scanlines and glow. Used when
    /// `/crt on` forces the tube over a theme that ships no style of its own.
    pub const DEFAULT: CrtStyle = CrtStyle {
        curvature: 0.0,
        scanline: 0.18,
        glow: 0.55,
        glow_radius: 6.0,
        corner: 0.0,
        flicker: 0.06,
    };
}
