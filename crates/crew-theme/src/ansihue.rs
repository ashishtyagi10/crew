//! Whose red is red: the six chromatic ANSI hues each palette separates its
//! slots by.
//!
//! crew's own palettes share [`crate::ansi::HUES`], crew's measured idea of
//! red, green and the rest. The classics are different: each is a scheme
//! people know by its shell colours as much as by its page — gruvbox's "blue"
//! is a grey teal, monokai's red is a hot pink, solarized's magenta leans
//! red. Drawn in crew's hues, `/theme gruvbox` showed gruvbox's page under
//! crew's `ls` and crew's syntax colours, which is half a gruvbox.
//!
//! So a classic declares its own six hues, measured in OKLCH from the
//! scheme's published terminal colours (the bright row where the scheme
//! draws text with it, as gruvbox does). Only the HUE is taken. Contrast and
//! chroma stay the house's ([`crate::ansi::ANSI_DARK`]), so every slot is as
//! legible as on any other palette and none shouts louder than another —
//! everforest's own chroma, for one, is too soft for its green and aqua to
//! stay apart at one contrast.
//!
//! Four schemes needed a nudge. At one contrast the bright row sits high in
//! lightness, where sRGB runs short of chroma, and two neighbours closed to
//! under `ansi_tests::SEPARABLE`: solarized's, everforest's and gruvbox's
//! red and magenta, Tokyo Night's cyan and magenta. Each pair was pushed
//! apart half a degree at a time until every slot cleared it with a margin —
//! 5.5° at most, short of anything that reads as a different colour.
//!
//! Three of crew's own palettes have hues of their own too: `neon`,
//! `amethyst` and `blossom`, whose characters are colours (a neon sign, a
//! gemstone, a cherry tree in spring) that
//! crew's median six flattened into the same shell output every other
//! palette prints.
use crate::ThemeId;

/// Tokyo Night: `#f7768e #9ece6a #e0af68 #7aa2f7 #bb9af7 #7dcfff`.
const TOKYO_NIGHT: [f32; 6] = [10.3, 130.1, 75.4, 264.2, 303.5, 231.7];
/// Solarized: `#dc322f #859900 #b58900 #268bd2 #d33682 #2aa198`.
const SOLARIZED: [f32; 6] = [30.6, 118.6, 85.7, 244.9, 352.4, 187.4];
/// Everforest: `#e67e80 #a7c080 #dbbc7f #7fbbb3 #d699b6 #83c092`.
const EVERFOREST: [f32; 6] = [21.6, 125.8, 83.7, 185.5, 347.2, 151.4];
/// Gruvbox (bright row): `#fb4934 #b8bb26 #fabd2f #83a598 #d3869b #8ec07c`.
const GRUVBOX: [f32; 6] = [35.9, 110.8, 83.0, 169.8, 356.7, 137.7];
/// Monokai: `#f92672 #a6e22e #e6db74 #66d9ef #ae81ff #a1efe4`.
const MONOKAI: [f32; 6] = [7.5, 127.3, 103.2, 212.0, 298.1, 184.8];

/// Neon (2026-10-29): crew's six turned toward the brief's neons — a pink
/// red, electric blue, violet magenta and a cooler cyan; green and gold stay
/// where they read brightest at the house contrast (a lemon yellow at this
/// lightness comes out olive).
const NEON: [f32; 6] = [6.0, 142.0, 88.0, 262.0, 306.0, 210.0];
/// Amethyst (2026-10-29): jewel tones round a royal-purple page — rose,
/// mint, amber, periwinkle, orchid and sky — rather than crew's six, which
/// on purple read as somebody else's palette.
const AMETHYST: [f32; 6] = [12.0, 170.0, 65.0, 268.0, 330.0, 215.0];

/// Blossom (2026-10-29): spring round a cherry tree — coral, leaf green,
/// apricot, sky, sakura pink and mint.
const BLOSSOM: [f32; 6] = [25.0, 138.0, 72.0, 245.0, 342.0, 180.0];

/// The hue of each chromatic slot on `id`, indexed by ANSI slot minus one:
/// red, green, yellow, blue, magenta, cyan.
pub fn hues(id: ThemeId) -> [f32; 6] {
    match id {
        ThemeId::NeonDark => NEON,
        ThemeId::Amethyst => AMETHYST,
        ThemeId::Blossom => BLOSSOM,
        ThemeId::TokyoNight => TOKYO_NIGHT,
        ThemeId::Solarized => SOLARIZED,
        ThemeId::Everforest => EVERFOREST,
        ThemeId::Gruvbox => GRUVBOX,
        ThemeId::Monokai => MONOKAI,
        _ => crate::ansi::HUES,
    }
}
