//! Where a spill is written, and how many are kept: the file half of
//! `spill`, which decides when to write and what the agent is told.
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Where spills go, relative to the working directory.
pub(super) const DIR: &str = ".crew/out";

/// Spill files kept. Thirty is a long task's worth of big results; a file is
/// only worth keeping while the agent that was told about it may still ask.
pub(super) const KEEP: usize = 30;

/// What `.crew/out/.gitignore` says: everything here, itself included.
const IGNORE: &str = "# crew's saved tool output (see docs/CREW.md): never committed\n*\n";

/// Write `body` to a new file in `root/.crew/out/` and return its path
/// relative to `root`, or `None` on any IO error, leaving no partial file.
pub(super) fn write(root: &Path, tool: &str, body: &str) -> Option<String> {
    static SEQ: AtomicU64 = AtomicU64::new(1);
    let dir = root.join(DIR);
    std::fs::create_dir_all(&dir).ok()?;
    let ignore = dir.join(".gitignore");
    if !ignore.exists() {
        std::fs::write(&ignore, IGNORE).ok()?;
    }
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    // `create_new`, so a second broker in the same second never overwrites
    // a file the first one has already pointed an agent at.
    let (name, mut file) = (0..8).find_map(|_| {
        let name = format!(
            "{tool}-{}-{}.txt",
            stamp(secs),
            SEQ.fetch_add(1, Ordering::Relaxed)
        );
        let file = std::fs::File::create_new(dir.join(&name)).ok()?;
        Some((name, file))
    })?;
    if file.write_all(body.as_bytes()).is_err() {
        let _ = std::fs::remove_file(dir.join(&name));
        return None;
    }
    prune(&dir);
    Some(format!("{DIR}/{name}"))
}

/// Delete all but the newest [`KEEP`] spills. Newest by name — stamp, then
/// sequence — rather than by mtime, which ties within a second on some disks.
fn prune(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut spills: Vec<_> = entries
        .flatten()
        .filter_map(|e| Some((order(&e.file_name().to_string_lossy())?, e.path())))
        .collect();
    spills.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, path) in spills.into_iter().skip(KEEP) {
        let _ = std::fs::remove_file(path);
    }
}

/// A spill's place in time from its name, `<tool>-<date>-<time>-<seq>.txt`,
/// or `None` for a file that is not one, which pruning leaves alone.
pub(super) fn order(name: &str) -> Option<(u64, u64, u64)> {
    let mut parts = name.strip_suffix(".txt")?.rsplitn(4, '-');
    let seq = parts.next()?.parse().ok()?;
    let time = parts.next()?.parse().ok()?;
    let date = parts.next()?.parse().ok()?;
    parts.next()?;
    Some((date, time, seq))
}

/// `secs` since the epoch as `YYYYMMDD-HHMMSS` in UTC: sorts as text, reads
/// at a glance, and needs no time-zone crate for one file name. The date is
/// Howard Hinnant's `civil_from_days`.
pub(super) fn stamp(secs: u64) -> String {
    let (z, t) = ((secs / 86_400) as i64 + 719_468, secs % 86_400);
    let (era, doe) = (z.div_euclid(146_097), z.rem_euclid(146_097));
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    let (h, min, s) = (t / 3_600, t / 60 % 60, t % 60);
    format!("{y:04}{m:02}{d:02}-{h:02}{min:02}{s:02}")
}
