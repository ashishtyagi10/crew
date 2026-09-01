//! Walking a directory tree and measuring it: the recursive descent a scan
//! runs on its worker thread, one directory's size, and laying the children
//! out as treemap tiles.
//!
//! Split from [`crate::diskpane`] for the line cap, along the line between the
//! pane and the filesystem work behind it.
use crate::diskpane::*;
pub(crate) use crate::disktile::*;
use crate::plot::treemap;
use std::path::Path;
use std::sync::atomic::Ordering;

/// Walk `root`'s children, publishing each child's total as it completes.
pub(crate) fn walk(root: &Path, scan: &Scan) {
    let Ok(dir) = std::fs::read_dir(root) else {
        scan.done.store(true, Ordering::Relaxed);
        return;
    };
    for entry in dir.flatten() {
        if scan.cancel.load(Ordering::Relaxed) {
            return;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let meta = entry.metadata().ok();
        let is_dir = meta.as_ref().is_some_and(|m| m.is_dir());
        let bytes = if is_dir {
            dir_size(&entry.path(), scan)
        } else {
            scan.seen.fetch_add(1, Ordering::Relaxed);
            meta.map(|m| m.len()).unwrap_or(0)
        };
        if let Ok(mut list) = scan.children.lock() {
            list.push(Child {
                name,
                bytes,
                is_dir,
            });
        }
    }
    scan.done.store(true, Ordering::Relaxed);
}

/// Bytes under `path`, following no symlinks (a link counts as its own tiny
/// size, not as the tree it points at — otherwise one link into `/` makes the
/// whole map a lie).
pub(crate) fn dir_size(path: &Path, scan: &Scan) -> u64 {
    let mut total = 0;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if scan.cancel.load(Ordering::Relaxed) {
            return total;
        }
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let Ok(meta) = e.metadata() else { continue };
            if meta.is_dir() {
                stack.push(e.path());
            } else if meta.is_file() {
                total += meta.len();
                scan.seen.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    total
}

/// Tiles for this pane's children, in cell coordinates.
pub fn tiles(children: &[Child], cols: u16, rows: u16) -> Vec<treemap::Tile> {
    let values: Vec<f64> = children.iter().map(|c| c.bytes as f64).collect();
    treemap::layout(map_rect(cols, rows), &values)
}
