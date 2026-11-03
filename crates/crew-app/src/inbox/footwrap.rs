//! The foot's keys laid into lines: whole groups to a line where they fit
//! (the user, 2026-11-03: "improve the look and feel of the button bar" —
//! groups broke anywhere, `s star` ending one line and `m read/unread`
//! starting the next), a quiet `·` between the groups on a line.
use super::{Key, Keys, GAP};
use crate::chatwidth::str_w;

/// Between two keys of a group.
const SEP: &str = "  ";
/// Between two groups on a line: the gap, its dot in the middle.
pub(crate) const DOT: &str = "\u{b7}";
const BETWEEN: usize = 3;
/// What stands for the keys a short pane leaves out.
pub(crate) const MORE: Key = ("?", "keys");

/// What a line holds at a column: a key, or the dot between two groups.
pub(crate) enum Placed {
    Key(Key),
    Dot,
}

/// A key's columns: its keycap, a space, what it does.
pub(crate) fn key_w(key: Key) -> usize {
    str_w(key.0) + 1 + str_w(key.1)
}

/// A line's columns, its groups' gaps included.
pub(crate) fn width(line: &[Key]) -> usize {
    let mut w = 0;
    for (i, &k) in line.iter().enumerate() {
        w += match (k == GAP, i) {
            (true, _) => BETWEEN,
            (false, 0) => key_w(k),
            // A key just after a gap: the gap has its own air.
            (false, _) if line[i - 1] == GAP => key_w(k),
            (false, _) => str_w(SEP) + key_w(k),
        };
    }
    w
}

/// Each key on `line` with the column it starts at, and each gap's dot.
pub(crate) fn placed(line: &[Key]) -> Vec<(u16, Placed)> {
    let mut out = Vec::new();
    for (i, &k) in line.iter().enumerate() {
        let col = 1 + width(&line[..i]) as u16;
        match k == GAP {
            true => out.push((col + (BETWEEN / 2) as u16, Placed::Dot)),
            false if i > 0 && line[i - 1] != GAP => {
                out.push((col + str_w(SEP) as u16, Placed::Key(k)))
            }
            false => out.push((col, Placed::Key(k))),
        }
    }
    out
}

/// `keys` into lines of at most `room` columns, in order: whole groups
/// when the lines a pane spares (`most`) hold them so, else packed key by
/// key, as every key showing comes first. Past `most` lines, the last ends
/// with `? keys` in place of what is left out.
pub(crate) fn lines(keys: Keys, room: usize, most: usize) -> Vec<Vec<Key>> {
    let whole = grouped(keys, room);
    let mut out = match whole.len() <= most {
        true => whole,
        false => packed(keys, room),
    };
    if out.len() > most {
        out.truncate(most);
        if let Some(last) = out.last_mut() {
            while !last.is_empty() && width(last) + str_w(SEP) + key_w(MORE) > room {
                last.pop();
            }
            while last.last() == Some(&GAP) {
                last.pop();
            }
            last.push(MORE);
        }
    }
    out
}

/// Key by key, each on the line it fits: a group's dot where it falls,
/// never ending a line.
fn packed(keys: Keys, room: usize) -> Vec<Vec<Key>> {
    let mut out: Vec<Vec<Key>> = Vec::new();
    for &k in keys {
        let fits = |line: &Vec<Key>| {
            let mut with = line.clone();
            with.push(k);
            width(&with) <= room
        };
        match out.last_mut() {
            Some(line) if !line.is_empty() && fits(line) => line.push(k),
            _ if k == GAP => {}
            _ => out.push(vec![k]),
        }
    }
    for line in &mut out {
        while line.last() == Some(&GAP) {
            line.pop();
        }
    }
    out
}

/// A group goes on the line it starts on when it fits there whole, else on
/// a line of its own, and only a group wider than a line breaks.
fn grouped(keys: &[Key], room: usize) -> Vec<Vec<Key>> {
    let mut out: Vec<Vec<Key>> = Vec::new();
    for group in keys.split(|&k| k == GAP).filter(|g| !g.is_empty()) {
        let fits = |line: &Vec<Key>| {
            let mut with = line.clone();
            with.push(GAP);
            with.extend_from_slice(group);
            width(&with) <= room
        };
        match out.last_mut() {
            Some(line) if fits(line) => {
                line.push(GAP);
                line.extend_from_slice(group);
                continue;
            }
            _ => out.push(Vec::new()),
        }
        for &k in group {
            let line = out.last_mut().expect("a line was just begun");
            match line.is_empty() || width(line) + str_w(SEP) + key_w(k) <= room {
                true => line.push(k),
                false => out.push(vec![k]),
            }
        }
    }
    out
}
