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
    /// Scanline darkening weight (0 = no raster lines).
    pub scanline: f32,
    /// Bloom composite strength: the blurred bright-pass is added back in
    /// scaled by this, so it is the "how much does light bleed" knob.
    pub glow: f32,
    /// Bloom blur radius in HALF-RES pixels — the blur chain runs on a
    /// half-resolution target, so the full-res reach is roughly 2× this.
    pub glow_radius: f32,
    /// Brightness-wobble amplitude while a pane is streaming. Idle frames
    /// always run at 0 regardless (the static-tube determinism contract);
    /// this is only what the app dials in during activity.
    pub flicker: f32,
    /// The FILAMENT: how far the brightest thin strokes — glyphs, frame
    /// lines — burn toward white at their core, inside their coloured halo,
    /// the way a neon tube's gas glows white where the current runs and
    /// colours the glass around it. 0 keeps every stroke its own colour;
    /// wide fills never burn (see `crt.wgsl`).
    pub core: f32,
    /// The tube's raster, ETCHED into its glass rather than laid over the
    /// window: fine lines in the glass body, under the text, so the panel
    /// carries the old tube's texture and no glyph is striped by it. The
    /// fill alpha the lines add at their darkest; 0 is clear glass.
    pub etch: f32,
    /// A soft SHADOW behind the tube's strokes, for a see-through window:
    /// how much of the desktop it dims, at most, behind dense text — so the
    /// phosphor still reads over a bright wallpaper while the glass between
    /// the lines stays clear. It only ever raises the window's alpha around
    /// what is already solid (glyphs, frames, fills); an opaque window has
    /// nothing behind it to dim, so 0 and any value draw the same there.
    pub shade: f32,
}

impl CrtStyle {
    /// Every tube's [`Self::shade`] (2026-10-06, after the tubes went sheer:
    /// "add the soft text shadow too"). 0.4 at first, halved the same day:
    /// "Shadow behind the lines and text are very strong, reduce them".
    pub const TUBE_SHADE: f32 = 0.2;

    /// The white-text glass's [`Self::shade`] (2026-10-09, the user: "bright
    /// white would look great on glass"): white words over a clear pane need
    /// the desktop behind them dimmed a little, as the iPhone does under its
    /// white labels. A touch more than a tube's, which only has to lift a
    /// glowing phosphor — kept gentle all the same, after "very strong".
    pub const GLASS_SHADE: f32 = 0.25;

    /// Today's look before the per-theme split: a flat phosphor panel
    /// (no warp, no bezel) with moderate scanlines and glow. Used when
    /// `/crt on` forces the tube over a theme that ships no style of its own.
    pub const DEFAULT: CrtStyle = CrtStyle {
        scanline: 0.18,
        glow: 0.55,
        glow_radius: 6.0,
        flicker: 0.06,
        core: 0.0,
        etch: 0.0,
        shade: 0.0,
    };
}
