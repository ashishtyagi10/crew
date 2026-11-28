//! Sixel colour registers: the VT340 defaults and the two ways a stream
//! defines its own (`#Pc;2;R;G;B` percent, `#Pc;1;H;L;S` hue/lightness/
//! saturation).

/// The VT340's 16 default registers (VT330/VT340 Programmer Reference Manual,
/// table 2-3), as sRGB bytes — the values foot ships, which are the table's
/// percentages rounded. Registers 16..255 start black, as in xterm and foot.
const DEFAULT: [[u8; 3]; 16] = [
    [0x00, 0x00, 0x00],
    [0x33, 0x33, 0xcc],
    [0xcc, 0x21, 0x21],
    [0x33, 0xcc, 0x33],
    [0xcc, 0x33, 0xcc],
    [0x33, 0xcc, 0xcc],
    [0xcc, 0xcc, 0x33],
    [0x87, 0x87, 0x87],
    [0x42, 0x42, 0x42],
    [0x54, 0x54, 0x99],
    [0x99, 0x42, 0x42],
    [0x54, 0x99, 0x54],
    [0x99, 0x54, 0x99],
    [0x54, 0x99, 0x99],
    [0x99, 0x99, 0x54],
    [0xcc, 0xcc, 0xcc],
];

/// All 256 registers at their power-on values.
pub(super) fn palette() -> [[u8; 3]; 256] {
    let mut p = [[0u8; 3]; 256];
    p[..16].copy_from_slice(&DEFAULT);
    p
}

/// `#Pc;Pu;Px;Py;Pz`'s colour, or `None` for a `Pu` that is neither 1 nor 2.
/// Components past their range are clamped: a garbage stream still draws.
pub(super) fn define(pu: u32, x: u32, y: u32, z: u32) -> Option<[u8; 3]> {
    let pct = |n: u32| (n.min(100) as f32) / 100.0;
    match pu {
        2 => Some([pct(x), pct(y), pct(z)].map(|c| (c * 255.0).round() as u8)),
        1 => Some(hls(x.min(360), pct(y), pct(z))),
        _ => None,
    }
}

/// HLS with DEC's hue origin: blue at 0°, red at 120°, green at 240° — the
/// usual wheel turned by 240° (foot's `(hue + 240) % 360`), so a stream that
/// says `#1;1;120;50;100` gets red, not green.
fn hls(dec_hue: u32, l: f32, s: f32) -> [u8; 3] {
    let h = ((dec_hue + 240) % 360) as f32 / 60.0;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    [r, g, b].map(|v| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8)
}
