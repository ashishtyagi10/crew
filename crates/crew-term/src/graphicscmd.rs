//! One graphics command, parsed out of its `key=value,…;payload` form.
//!
//! The kitty protocol is a large vocabulary — ids, placements, z-order,
//! animation frames, unicode placeholders, deletion by half a dozen criteria.
//! What crew reads is the part every producer emits: *here is a picture, put
//! it here*, in one piece or in chunks, as bytes or as a file on disk.
//!
//! Everything unrecognised is kept out of the terminal rather than guessed at:
//! an unparseable command produces no image, and the sequence has already been
//! taken out of the byte stream, so the worst case is a picture that does not
//! appear — never a screenful of escape-sequence text.
use base64::Engine;

#[path = "graphicsiterm.rs"]
mod iterm;

/// A picture (or an instruction about one) as it arrived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageCmd {
    /// `a=`: `T` transmit and display, `t` transmit only, `p` place, `d`
    /// delete. Defaults to `t`, as the protocol says.
    pub action: u8,
    /// `f=`: 100 for a PNG file, 24 for raw RGB, 32 for raw RGBA.
    pub format: u32,
    /// `t=`: `d` the payload is the image, `f`/`t` the payload is a path.
    pub medium: u8,
    /// `s=`,`v=`: pixel size, which raw formats carry no header for.
    pub px: (u32, u32),
    /// `c=`,`r=`: how many cells the sender wants it drawn across. `0` means
    /// "as big as it comes".
    pub cells: (u16, u16),
    /// `i=`: the sender's id for this image, used to place or delete it later.
    pub id: u32,
    /// `d=`: which pictures a delete (`a=d`) takes — `a` all (the default),
    /// `i` the one named by `i=`; upper case also frees the data, which crew
    /// keeps none of.
    pub delete: u8,
    /// `m=1`: more chunks follow, and this payload is a fragment.
    pub more: bool,
    /// `C=1` (iTerm2: `doNotMoveCursor=1`): leave the cursor where it is. A
    /// full-screen program positions the cursor itself and places a picture
    /// there; moving past it would line-feed, and at the bottom scroll its
    /// whole layout.
    pub stay: bool,
    /// `q=`: 1 silences the `OK` answer, 2 silences errors too — a sender
    /// that does not read replies must not have them typed into its input.
    pub quiet: u8,
    /// The payload is a sixel stream (`DCS P1;P2;P3 q … ST`), not one of
    /// kitty's formats. A flag of its own rather than a new `f=` value so no
    /// kitty sender can name it: `a=q` and [`Self::supported`] keep answering
    /// for kitty's formats only.
    pub sixel: bool,
    /// The decoded payload: image bytes, or a path.
    pub data: Vec<u8>,
}

impl Default for ImageCmd {
    fn default() -> Self {
        Self {
            action: b't',
            format: 32,
            medium: b'd',
            px: (0, 0),
            cells: (0, 0),
            id: 0,
            delete: b'a',
            more: false,
            stay: false,
            quiet: 0,
            sixel: false,
            data: Vec::new(),
        }
    }
}

impl ImageCmd {
    /// A sixel picture at the cursor: `body` is everything between `ESC P` and
    /// the terminator (params, `q`, data). Like iTerm2's, it is a request to
    /// display, with no id, so nothing is ever written back.
    pub(crate) fn sixel(body: Vec<u8>) -> Self {
        Self {
            action: b'T',
            format: 0,
            sixel: true,
            data: body,
            ..Self::default()
        }
    }

    /// Parse the body of an `APC G` sequence: everything between `ESC _ G` and
    /// the terminator. `None` when it is not a `G` command at all.
    pub(crate) fn parse(body: &[u8]) -> Option<Self> {
        let body = body.strip_prefix(b"G")?;
        let (keys, payload) = match body.iter().position(|&b| b == b';') {
            Some(i) => (&body[..i], &body[i + 1..]),
            None => (body, &[][..]),
        };
        let mut cmd = Self::default();
        for pair in keys.split(|&b| b == b',').filter(|p| !p.is_empty()) {
            let i = pair.iter().position(|&b| b == b'=')?;
            let (k, v) = (&pair[..i], &pair[i + 1..]);
            let num = || std::str::from_utf8(v).ok()?.parse::<u32>().ok();
            match k {
                b"a" => cmd.action = *v.first()?,
                b"t" => cmd.medium = *v.first()?,
                b"f" => cmd.format = num()?,
                b"s" => cmd.px.0 = num()?,
                b"v" => cmd.px.1 = num()?,
                b"c" => cmd.cells.0 = num()?.min(u16::MAX.into()) as u16,
                b"r" => cmd.cells.1 = num()?.min(u16::MAX.into()) as u16,
                b"i" => cmd.id = num()?,
                b"d" => cmd.delete = *v.first()?,
                b"m" => cmd.more = num()? == 1,
                b"C" => cmd.stay = num()? == 1,
                b"q" => cmd.quiet = num()?.min(2) as u8,
                // An unknown key is not a reason to drop a picture: the
                // protocol grows, and everything crew needs is above.
                _ => {}
            }
        }
        cmd.data = base64::engine::general_purpose::STANDARD
            .decode(payload)
            .ok()?;
        Some(cmd)
    }

    /// Whether this command puts a picture on the screen now. `t` (transmit
    /// only) stores for a later `p`, which crew does not keep a store for, so
    /// only the display actions count.
    pub fn displays(&self) -> bool {
        matches!(self.action, b'T' | b'p')
    }

    /// Whether this command deletes what is on screen.
    pub fn deletes(&self) -> bool {
        self.action == b'd'
    }

    /// Whether crew can draw what this command describes — the answer to an
    /// `a=q` probe. A PNG or raw pixels, arriving as bytes or as a file: the
    /// shapes `crew-app`'s decoder handles. Shared memory (`t=s`) and the
    /// compressed raw formats are not among them, and saying so is better
    /// than accepting a picture that will never appear.
    pub fn supported(&self) -> bool {
        self.sixel
            || matches!(self.format, 100 | 24 | 32) && matches!(self.medium, b'd' | b'f' | b't')
    }

    /// The payload read as a filesystem path, for `t=f` / `t=t`.
    pub fn path(&self) -> Option<std::path::PathBuf> {
        matches!(self.medium, b'f' | b't')
            .then(|| std::str::from_utf8(&self.data).ok())
            .flatten()
            .map(std::path::PathBuf::from)
    }

    /// The pixel size this command's picture will decode to, without decoding it:
    /// a PNG says so in the first chunk of its header, and a raw transmission says
    /// so in its keys. Needed *before* the bytes are handed to a worker, because
    /// how many rows the picture occupies decides how many the terminal has to
    /// scroll to make room for it.
    pub fn pixel_size(&self) -> Option<(u32, u32)> {
        if self.sixel {
            return crate::sixel::size(&self.data);
        }
        if self.format != 100 {
            return (self.px.0 > 0 && self.px.1 > 0).then_some(self.px);
        }
        let d = &self.data;
        // `\x89PNG\r\n\x1a\n` then an 8-byte IHDR chunk header, then the two
        // big-endian dimensions.
        if d.len() < 24 || !d.starts_with(b"\x89PNG\r\n\x1a\n") {
            return None;
        }
        let n = |o: usize| u32::from_be_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]]);
        Some((n(16), n(20)))
    }
}

#[cfg(test)]
#[path = "graphicscmd_tests.rs"]
mod graphicscmd_tests;
