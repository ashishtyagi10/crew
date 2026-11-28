//! iTerm2's spelling of the picture command, `OSC 1337 ; File=…`. Split from
//! `graphicscmd.rs` (child module: it reads the same fields) for the line cap.
use base64::Engine;

use crate::graphicscmd::ImageCmd;

impl ImageCmd {
    /// Parse the body of an iTerm2 `OSC 1337 ; File=…` sequence: everything
    /// after the `1337;File=` the splitter matched, which is
    /// `key=value;…:<base64>`.
    ///
    /// A different spelling of the same request — several tools write this one
    /// and not kitty's — so it lands in the same command.
    pub(crate) fn parse_iterm(body: &[u8]) -> Option<Self> {
        let i = body.iter().position(|&b| b == b':')?;
        let (keys, payload) = (&body[..i], &body[i + 1..]);
        let mut cmd = Self {
            action: b'T',
            format: 100,
            medium: b'd',
            ..Self::default()
        };
        let mut inline = false;
        for pair in keys.split(|&b| b == b';').filter(|p| !p.is_empty()) {
            let Some(eq) = pair.iter().position(|&b| b == b'=') else {
                continue;
            };
            let k = std::str::from_utf8(&pair[..eq]).ok()?.to_ascii_lowercase();
            let v = std::str::from_utf8(&pair[eq + 1..]).ok()?;
            match k.as_str() {
                "inline" => inline = v == "1",
                "width" => size_arg(v, &mut cmd.cells.0, &mut cmd.px.0),
                "height" => size_arg(v, &mut cmd.cells.1, &mut cmd.px.1),
                "donotmovecursor" => cmd.stay = v == "1",
                _ => {}
            }
        }
        // `inline=0` (the default) means *download this file*, which a
        // terminal has no business doing on a program's say-so.
        if !inline {
            return None;
        }
        cmd.data = base64::engine::general_purpose::STANDARD
            .decode(
                payload
                    .iter()
                    .copied()
                    .filter(|b| !b.is_ascii_whitespace())
                    .collect::<Vec<u8>>(),
            )
            .ok()?;
        // The pixel size comes from the file's own header; `width=Npx` is a
        // request about the DISPLAY size, which crew answers by fitting.
        cmd.px = (0, 0);
        Some(cmd)
    }
}

/// iTerm2's `width=`/`height=`: a bare number is cells, `Npx` is pixels, and
/// `N%` is a share of the window — which crew has no answer for at parse time
/// and treats as "as big as it comes".
fn size_arg(v: &str, cells: &mut u16, px: &mut u32) {
    if let Some(n) = v.strip_suffix("px") {
        *px = n.parse().unwrap_or(0);
    } else if !v.ends_with('%') {
        *cells = v.parse().unwrap_or(0);
    }
}
