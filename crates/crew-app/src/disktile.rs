//! How one treemap tile READS: the colour it takes, how far it fades, the ink
//! its label needs against that background, and the shortened path that fits
//! in it.
//!
//! Split from [`crate::diskpane`] for the line cap, along the line between
//! the tree the pane walked and the ink each tile gets.
use crate::diskpane::*;
use crate::plot::treemap;
use crew_render::CellView;
use std::path::Path;

/// Bytes as `4.2G`, `812M`, `9.1k`, `640B` — four characters wherever
/// possible, because most of them are written inside a tile.
pub fn bytes(n: u64) -> String {
    const K: f64 = 1024.0;
    let b = n as f64;
    match n {
        0..=1023 => format!("{n}B"),
        _ if b < K * K => format!("{:.0}k", b / K),
        _ if b < K * K * K => format!("{:.0}M", b / (K * K)),
        _ if b < K * K * K * K => format!("{:.1}G", b / (K * K * K)),
        _ if b < K * K * K * K * K => format!("{:.1}T", b / (K * K * K * K)),
        // No real filesystem gets here, but a tile's label must never grow
        // past the box it is written in whatever the number says.
        _ => format!("{:.1}P", b / (K * K * K * K * K)),
    }
}

/// Rows the header claims above the map.
pub(crate) const HEAD: u16 = 2;

/// The map's rect inside a `cols`×`rows` pane, in cells.
pub(crate) fn map_rect(cols: u16, rows: u16) -> (f32, f32, f32, f32) {
    (
        1.0,
        f32::from(HEAD),
        f32::from(cols.saturating_sub(2)),
        f32::from(rows.saturating_sub(HEAD + 1)),
    )
}

pub(crate) fn put(
    out: &mut Vec<CellView>,
    s: &str,
    col: u16,
    row: u16,
    fg: (u8, u8, u8),
    cols: u16,
) {
    for (i, ch) in s.chars().enumerate() {
        let col = col + i as u16;
        if col >= cols {
            break;
        }
        out.push(CellView {
            col,
            row,
            c: ch,
            fg,
            bg: crew_theme::theme().page_bg,
            ..Default::default()
        });
    }
}

/// The ink a tile's label is written in: the page's own background pushed
/// until it clears the reading floor against `bg`.
///
/// `enforced`, not `against`: a tile's fill is a colour the app composited,
/// not one anybody picked against this page, and the pool has hues that top
/// out short of the floor at every lightness — a file tile at 0.55 alpha over
/// a dark page reached 4.34 and stopped there. `enforced` gives up chroma
/// rather than the floor, so every tile on the map is readable, not most.
pub(crate) fn label_ink(bg: (u8, u8, u8)) -> (u8, u8, u8) {
    crew_theme::readable::enforced(
        crew_theme::theme().page_bg,
        bg,
        crew_theme::contrast::text_floor(),
    )
}

/// Columns the header keeps for the path before it gives up and shows the
/// reading alone — fewer than this and the leading `\u{2026}` is most of it.
pub(crate) const MIN_PATH_W: u16 = 8;

/// The colour a tile's label is read against: its fill composited over the
/// page at the alpha the fill is actually drawn with, which is the background
/// the eye sees — not the raw pool colour, which a 0.55-alpha file tile never
/// shows.
pub(crate) fn tile_bg(color: (u8, u8, u8), is_dir: bool, selected: bool) -> (u8, u8, u8) {
    let page = crew_theme::theme().page_bg;
    let a = tile_alpha(is_dir, selected);
    let mix = |c: u8, p: u8| (f32::from(c) * a + f32::from(p) * (1.0 - a)).round() as u8;
    (
        mix(color.0, page.0),
        mix(color.1, page.1),
        mix(color.2, page.2),
    )
}

/// Whether two tiles share an edge or a corner — laid out edge to edge, so
/// "touching" is what a treemap's neighbours always are.
pub(crate) fn touches(a: &treemap::Tile, b: &treemap::Tile) -> bool {
    const E: f32 = 0.01;
    a.x < b.x + b.w + E && b.x < a.x + a.w + E && a.y < b.y + b.h + E && b.y < a.y + a.h + E
}

/// A colour per tile, chosen so that no two tiles that touch share one.
///
/// The pool is six entries picked by hashing the name, which on a directory
/// with eight children collides by the pigeonhole principle long before it
/// collides by bad luck: in the repo's own root `crates` and `.git` came out
/// byte-identical, and so did `target` and `docs`. Two neighbouring tiles the
/// same colour read as one region, which is the one thing a map of areas is
/// for. On a single-phosphor tube, where the whole pool is four shades of the
/// same green, it is the only thing keeping the regions apart at all.
///
/// The name still picks first, so a directory keeps its colour when you
/// rescan the parent it is in; only a tile that would touch a twin steps
/// along the pool to the next free entry.
pub(crate) fn tile_colors(children: &[Child], tiles: &[treemap::Tile]) -> Vec<(u8, u8, u8)> {
    let pool: Vec<(u8, u8, u8)> = {
        let t = crew_theme::theme();
        let mut v: Vec<(u8, u8, u8)> = Vec::new();
        for c in &t.ansi[9..=14] {
            if !v.contains(c) {
                v.push(*c);
            }
        }
        v
    };
    let mut picked: Vec<Option<usize>> = vec![None; tiles.len()];
    for (i, tile) in tiles.iter().enumerate() {
        let Some(child) = children.get(tile.index) else {
            continue;
        };
        let first = pool
            .iter()
            .position(|&c| c == crate::chatroster::agent_color(&child.name))
            .unwrap_or(0);
        // The neighbours that already have one. Tiles are visited in the
        // layout's own order, so this is deterministic for a given listing.
        let taken: Vec<usize> = tiles
            .iter()
            .enumerate()
            .filter(|&(j, t)| j != i && touches(tile, t))
            .filter_map(|(j, _)| picked[j])
            .collect();
        picked[i] = Some(
            (0..pool.len())
                .map(|d| (first + d) % pool.len())
                // A tile with more neighbours than the pool has colours keeps
                // the one its name asked for: a repeat somewhere beats a
                // colour nothing chose.
                .find(|k| !taken.contains(k))
                .unwrap_or(first),
        );
    }
    picked.into_iter().map(|k| pool[k.unwrap_or(0)]).collect()
}

/// How solid a tile is drawn. Directories carry their colour; plain files sit
/// back, so a tree full of one big file still reads as different from a
/// subtree — and the picked tile is solid whichever it is.
pub(crate) fn tile_alpha(is_dir: bool, selected: bool) -> f32 {
    match (is_dir, selected) {
        (_, true) => 1.0,
        (true, false) => 0.85,
        (false, false) => 0.55,
    }
}

/// `~/code/crew` rather than `/Users/you/code/crew`.
pub(crate) fn short_path(p: &Path) -> String {
    let s = p.to_string_lossy();
    match dirs::home_dir() {
        Some(home) => match s.strip_prefix(home.to_string_lossy().as_ref()) {
            Some(rest) => format!("~{rest}"),
            None => s.into_owned(),
        },
        None => s.into_owned(),
    }
}
