//! The scanner's vocabulary: its states, what it hands back, and its limits.
//! Split from `graphics.rs` for the 200-line cap.

pub(super) const ESC: u8 = 0x1b;
pub(super) const BEL: u8 = 0x07;

/// Cap on one accumulated payload. Kitty sends at most 4096 base64 bytes per
/// chunk and a real screenshot is a few hundred of them; this is the guard
/// against a sequence that never terminates growing the buffer forever.
pub(super) const MAX_PAYLOAD: usize = 16 * 1024 * 1024;

use crate::graphicscmd::ImageCmd;

/// What the OSC prefix probe has decided so far.
pub(super) enum Probe {
    More,
    Picture,
    No,
}

#[derive(Default, Clone, Copy, PartialEq)]
pub(super) enum St {
    #[default]
    Ground,
    /// Saw `ESC`, waiting to learn whether this is an APC.
    Esc,
    /// Inside `ESC _ …`.
    Apc,
    /// Inside an APC and saw `ESC` — one byte from the string terminator.
    ApcEsc,
    /// Saw `ESC ]` and is reading just enough of the OSC to know whether it
    /// is iTerm2's picture (`1337;File=`) or somebody else's sequence.
    OscProbe,
    /// Inside an `ESC ] 1337 ; File=…` picture.
    Osc,
    /// Inside one, and saw `ESC` — one byte from the string terminator.
    OscEsc,
    /// Saw `ESC P`, reading digits and `;` — a sixel if a `q` follows.
    DcsProbe,
    /// Inside a sixel (`buf` holds `params q data`).
    Sixel,
    /// Inside one, and saw `ESC`: `\` ends it, anything else aborts it.
    SixelEsc,
}

/// Longest DCS header that can still be sixel's (`P1;P2;P3`).
pub(super) const DCS_PROBE: usize = 16;

/// How much of an OSC is read before deciding it is not a picture. Exactly
/// the length of the prefix that says it is one.
pub(super) const OSC_PROBE: &[u8] = b"1337;File=";

/// One piece of the split stream.
pub(crate) enum Seg<'a> {
    /// Bytes for the ANSI parser, exactly as they arrived.
    Bytes(&'a [u8]),
    /// A complete graphics command, at this point in the stream.
    Image(ImageCmd),
    /// Bytes held back across a chunk boundary that turned out not to start a
    /// picture after all — a lone `ESC`, or the first bytes of an OSC that is
    /// somebody else's. Its own variant because they are not a slice of the
    /// chunk being split.
    Held(Vec<u8>),
}
