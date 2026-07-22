//! `rclone` command construction and `lsjson` parsing for remote Far panels.
//! Everything here is pure — argv vectors and JSON→`Entry` mapping — so it is
//! unit-tested without a network or an installed `rclone`. The worker that
//! actually runs these argv lives alongside in `run` (Task 3).
use serde::Deserialize;

use super::list;
use super::location::Location;
use super::Entry;

pub(crate) fn argv_lsjson(loc: &Location) -> Vec<String> {
    vec!["lsjson".into(), loc.rclone_addr()]
}

pub(crate) fn argv_listremotes() -> Vec<String> {
    vec!["listremotes".into()]
}

pub(crate) fn argv_mkdir(loc: &Location) -> Vec<String> {
    vec!["mkdir".into(), loc.rclone_addr()]
}

/// `deletefile` for a single file (Drive trash on by default); `purge` removes
/// a directory and its contents.
pub(crate) fn argv_delete(loc: &Location, is_dir: bool) -> Vec<String> {
    let verb = if is_dir { "purge" } else { "deletefile" };
    vec![verb.into(), loc.rclone_addr()]
}

pub(crate) fn argv_copy(src: &Location, dst: &Location, is_dir: bool) -> Vec<String> {
    let verb = if is_dir { "copy" } else { "copyto" };
    vec![verb.into(), src.rclone_addr(), dst.rclone_addr()]
}

pub(crate) fn argv_move(src: &Location, dst: &Location, is_dir: bool) -> Vec<String> {
    let verb = if is_dir { "move" } else { "moveto" };
    vec![verb.into(), src.rclone_addr(), dst.rclone_addr()]
}

/// One row of `rclone lsjson` output (only the fields we use).
#[derive(Deserialize)]
struct LsRow {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Size")]
    size: i64,
    #[serde(rename = "IsDir")]
    is_dir: bool,
}

/// Parse `rclone lsjson` output for `loc` into a sorted `Entry` list, with the
/// synthetic `..` row prepended when `loc` has a parent.
pub(crate) fn parse_lsjson(json: &str, loc: &Location) -> Result<Vec<Entry>, String> {
    let rows: Vec<LsRow> = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    if loc.has_parent() {
        out.push(Entry {
            name: "..".into(),
            is_dir: true,
            is_parent: true,
            size: 0,
        });
    }
    let mut items: Vec<Entry> = rows
        .into_iter()
        .map(|r| Entry {
            name: r.name,
            is_dir: r.is_dir,
            is_parent: false,
            // Drive dirs report Size -1; normalise to 0 like the local reader.
            size: if r.is_dir || r.size < 0 {
                0
            } else {
                r.size as u64
            },
        })
        .collect();
    list::sort_entries(&mut items);
    out.extend(items);
    Ok(out)
}

#[cfg(test)]
#[path = "rclone_tests.rs"]
mod tests;
