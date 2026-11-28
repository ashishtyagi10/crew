//! Queries the parser never dispatches, answered from the raw pty bytes.
//!
//! - `CSI 16 t`, "report the cell size" (see `modelsize`): pictures are sized
//!   from it.
//! - XTVERSION, `CSI > q` / `CSI > 0 q`, "name yourself": over ssh or inside
//!   tmux `TERM_PROGRAM` never reaches the program, and this is how neovim,
//!   yazi, fzf's previews, notcurses and the kitty-graphics probes recognise
//!   the terminal before they turn on pictures, synchronized output or
//!   undercurl. vte has no arm for it, so without this it went unanswered.
//! - XTSMGRAPHICS colour-register count (256), for sixel tools.
//! - XTGETTCAP, `DCS + q <hex names> ST`, "what can you do" (see `modelcaps`).

/// The sequences answered here. None holds an ESC past its first byte, which
/// is what lets [`Queries::feed`] restart cleanly on any ESC.
const CELL_QUERY: &[u8] = b"\x1b[16t";
const XTVERSION: &[u8] = b"\x1b[>q";
const XTVERSION_0: &[u8] = b"\x1b[>0q";
/// XTSMGRAPHICS, "how many sixel colour registers?": `CSI ? 1 ; 1 ; 0 S`
/// (current) and `CSI ? 1 ; 4 ; 0 S` (maximum). The geometry query
/// (`? 2 ; 1 ; 0 S`) is a documented gap and stays unanswered.
const SIXEL_COLORS: &[u8] = b"\x1b[?1;1;0S";
const SIXEL_COLORS_MAX: &[u8] = b"\x1b[?1;4;0S";
const ALL: [&[u8]; 5] = [
    CELL_QUERY,
    XTVERSION,
    XTVERSION_0,
    SIXEL_COLORS,
    SIXEL_COLORS_MAX,
];

/// The XTVERSION reply: `DCS > | name version ST`, as xterm, kitty and
/// WezTerm send it.
pub(crate) fn xtversion_reply() -> String {
    format!("\x1bP>|crew {}\x1b\\", env!("CARGO_PKG_VERSION"))
}

/// Watches for [`ALL`]. Only the bytes matched so far are kept (at most a
/// query's length), so a query split across reads still answers.
#[derive(Default)]
pub(crate) struct Queries {
    matched: Vec<u8>,
    /// XTGETTCAP, stepped on the same bytes so replies keep the asked order.
    caps: modelcaps::Caps,
}

impl Queries {
    /// Scan a chunk; the replies owed for it, in the order asked.
    pub(crate) fn feed(&mut self, bytes: &[u8], (w, h): (u32, u32)) -> String {
        let mut out = String::new();
        for &b in bytes {
            if let Some(reply) = self.caps.step(b) {
                out.push_str(&reply);
            }
            self.matched.push(b);
            if !ALL.iter().any(|q| q.starts_with(&self.matched)) {
                // An ESC always starts a new sequence, whatever came before.
                self.matched.clear();
                if b == 0x1b {
                    self.matched.push(b);
                }
                continue;
            }
            let reply = match self.matched.as_slice() {
                CELL_QUERY => format!("\x1b[6;{h};{w}t"),
                XTVERSION | XTVERSION_0 => xtversion_reply(),
                // `Ps = 0` is success; 256 registers is what `sixel::decode` keeps.
                SIXEL_COLORS | SIXEL_COLORS_MAX => "\x1b[?1;0;256S".to_string(),
                _ => continue,
            };
            out.push_str(&reply);
            self.matched.clear();
        }
        out
    }
}

#[path = "modelcaps.rs"]
mod modelcaps;

#[cfg(test)]
#[path = "modelquery_tests.rs"]
mod tests;
