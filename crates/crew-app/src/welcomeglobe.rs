//! Procedurally rendered ASCII globe for the welcome screen: an orthographic,
//! rotating projection of a coarse embedded continent bitmap, shaded by a
//! fixed upper-left light so the sphere reads as a ball, not a flat map.
use crew_render::CellView;
use std::f32::consts::{PI, TAU};

/// Default globe box size in cells. 44 wide × 22 tall keeps the disc a
/// circle, not an oval, given the terminal's ~2:1 cell aspect (a cell is
/// twice as tall as wide).
pub const GLOBE_W: u16 = 44;
pub const GLOBE_H: u16 = GLOBE_W / 2;
/// Smallest globe box still worth drawing; below this `welcome.rs` falls
/// back to the spaced single-line "CREW".
pub const GLOBE_MIN_W: u16 = 16;
pub const GLOBE_MIN_H: u16 = GLOBE_MIN_W / 2;

const EARTH_W: usize = 48;
const EARTH_H: usize = 24;

/// Earth bitmap: 48 longitude columns × 24 latitude rows, 1 bit/cell, packed
/// MSB-first into the low 48 bits of each `u64` (bit 47 = column 0, bit 0 =
/// column 47). Row 0 is the north-pole band, row 23 the south-pole band.
/// Column 0 is an arbitrary longitude reference frame — `globe`'s `phase`
/// rotates the visible slice through it every frame, so its absolute
/// meaning never surfaces. Equirectangular, coarse continents (`#` = land):
///
/// ```text
/// ................................................
/// ......##########..#####.........................
/// ..####################........##################
/// .#####################..########################
/// ...##################.##########################
/// ......############....#########################.
/// ......###########.....######################....
/// .......#########......######################....
/// .......#########.....######################.....
/// ........#####........######################.....
/// ........#########....######################.....
/// ............#####.....##########.###.######.....
/// ..............######.....#######.....######.....
/// ...............#####......######.......######...
/// ...............#####......######.......######...
/// ..............#####.......######.......######...
/// ..............####........###..........######...
/// ..............###......................######.##
/// ..............###.............................##
/// ..............##................................
/// ................................................
/// ................................................
/// ................................................
/// ................................................
/// ```
///
/// Top to bottom: Arctic islands + Greenland; North America tapering through
/// the Central American isthmus into South America, which tapers to a point
/// (Patagonia) near the bottom. The Old World mass carries Europe merging
/// into Siberia/Asia (Eurasia is genuinely one landmass), Africa hanging
/// beneath Europe, India and Indonesia as separate tendrils off Asia's
/// southern edge, and Australia + New Zealand as islands lower-right.
#[rustfmt::skip]
const EARTH: [u64; EARTH_H] = [
    0x000000000000, 0x03FF3E000000, 0x3FFFFC03FFFF, 0x7FFFFCFFFFFF,
    0x1FFFFBFFFFFF, 0x03FFC3FFFFFE, 0x03FF83FFFFF0, 0x01FF03FFFFF0,
    0x01FF07FFFFE0, 0x00F807FFFFE0, 0x00FF87FFFFE0, 0x000F83FF77E0,
    0x0003F07F07E0, 0x0001F03F01F8, 0x0001F03F01F8, 0x0003E03F01F8,
    0x0003C03801F8, 0x0003800001FB, 0x000380000003, 0x000300000000,
    0x000000000000, 0x000000000000, 0x000000000000, 0x000000000000,
];

/// Land shading, brightest (fully lit) to dimmest (grazing/shadowed).
const LAND_CHARS: [char; 5] = ['#', '%', '*', '+', '='];
/// Sea shading, brightest to dimmest — dimmest is a space (bg shows through).
const SEA_CHARS: [char; 3] = ['·', '.', ' '];

/// True if latitude band `row` (`0..EARTH_H`) / longitude band `col`
/// (`0..EARTH_W`) is land. Out-of-range indices read as sea.
fn is_land(row: usize, col: usize) -> bool {
    if row >= EARTH_H || col >= EARTH_W {
        return false;
    }
    (EARTH[row] >> (EARTH_W - 1 - col)) & 1 != 0
}

/// Pick a shading character from `chars` (ordered brightest → dimmest) for
/// illumination `illum` in `[0, 1]`.
fn shade_char(chars: &[char], illum: f32) -> char {
    let n = chars.len();
    let idx = (((1.0 - illum.clamp(0.0, 1.0)) * n as f32) as usize).min(n - 1);
    chars[idx]
}

/// Unit-length light direction, fixed once: upper-left, tilted toward the
/// viewer, so the sphere reads with a highlight near its upper-left limb.
fn light_dir() -> (f32, f32, f32) {
    let (x, y, z) = (-0.5_f32, 0.6_f32, 0.65_f32);
    let len = (x * x + y * y + z * z).sqrt();
    (x / len, y / len, z / len)
}

/// Render a `w`×`h`-cell globe frame at rotation `phase` (radians) with its
/// top-left at `(top, left)`. Pure and deterministic: identical inputs
/// always produce identical cells, in the same order. Points outside the
/// projected disc emit no cell (the page shows through).
///
/// Projection: each cell maps to `(u, v)` in roughly `[-1, 1]`, normalized
/// by half-width and half-height separately so a `w × w/2` box (the 2:1 cell
/// aspect) reads as a circle. Points with `u²+v² > 1` are outside the disc.
/// Inside it, `zc = sqrt(1 - u² - v²)` completes a unit sphere point
/// `(u, v, zc)`; standard spherical unprojection gives `lat = asin(v)` and
/// `lon = atan2(u, zc)` (this holds for every `v`, not just the equator —
/// `zc` is exactly `cos(lat)` at any latitude). `phase` is added to `lon`
/// before wrapping to rotate the visible slice.
#[rustfmt::skip]
#[allow(clippy::too_many_arguments)] // geometry + three colours, all independent
pub fn globe(cells: &mut Vec<CellView>, top: u16, left: u16, w: u16, h: u16,
             phase: f32, land: (u8, u8, u8), sea: (u8, u8, u8), bg: (u8, u8, u8)) {
    if w == 0 || h == 0 { return; }
    let cx = (w - 1) as f32 / 2.0;
    let cy = (h - 1) as f32 / 2.0;
    let hw = w as f32 / 2.0;
    let hh = h as f32 / 2.0;
    let (lx, ly, lz) = light_dir();

    for row in 0..h {
        for col in 0..w {
            let u = (col as f32 - cx) / hw;
            let v = (cy - row as f32) / hh; // flip: row 0 (top) -> v > 0 (north)
            let d2 = u * u + v * v;
            if d2 > 1.0 { continue; } // outside the disc: page shows through

            let zc = (1.0 - d2).max(0.0).sqrt();
            let lat = v.clamp(-1.0, 1.0).asin();
            let lon = (u.atan2(zc) + phase).rem_euclid(TAU);

            let erow = (((PI / 2.0 - lat) / PI) * EARTH_H as f32) as usize;
            let erow = erow.min(EARTH_H - 1);
            let ecol = ((lon / TAU) * EARTH_W as f32) as usize % EARTH_W;

            let illum = (u * lx + v * ly + zc * lz).clamp(0.0, 1.0);
            let (c, fg) = if is_land(erow, ecol) {
                (shade_char(&LAND_CHARS, illum), land)
            } else {
                (shade_char(&SEA_CHARS, illum), sea)
            };
            cells.push(CellView { col: left + col, row: top + row, c, fg, bg, bold: false, italic: false });
        }
    }
}

#[cfg(test)]
#[path = "welcomeglobe_tests.rs"]
mod tests;
