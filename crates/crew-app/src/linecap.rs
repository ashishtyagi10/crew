//! The 200-line cap, enforced as a ratchet.
//!
//! The project rule is that no `.rs` file exceeds 200 lines, and when this was
//! written 166 of them did. A rule with that many exceptions is enforced by
//! nothing, which is how it got to 166.
//!
//! So this is not a cliff. `line-cap-debt.txt` lists the files already over,
//! with the length each was at, and the test fails if one of them GROWS, if a
//! file not on the list crosses the cap, or if a listed file drops under 200
//! without its row being deleted. Shrinking is free; finishing a file means
//! removing its row, after which it can never drift back.
//!
//! The debt is a data file rather than a table in here for one reason worth
//! stating: a 166-row table would put THIS file over the cap it enforces.

/// The cap, from the project guardrails.
#[cfg(test)]
pub(crate) const LINE_CAP: usize = 200;

/// Walk this crate's sources, yielding `(path relative to `crates/`, lines)`.
///
/// Paths, not file NAMES: the tree has a dozen `mod.rs` and several `keys.rs`,
/// and a debt list keyed by name marks all of them finished the moment the
/// shortest one is. That is not a hypothetical — it is what the first version
/// of this list did.
#[cfg(test)]
pub(crate) fn sources() -> Vec<(String, usize)> {
    fn walk(dir: &std::path::Path, out: &mut Vec<(String, usize)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                if let Ok(s) = std::fs::read_to_string(&p) {
                    let rel = p
                        .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                        .unwrap_or(&p)
                        .to_string_lossy()
                        .replace('\\', "/");
                    out.push((rel, s.lines().count()));
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .as_path(),
        &mut out,
    );
    out
}

/// The recorded debt: `(path, length when written)`.
#[cfg(test)]
pub(crate) fn debt() -> Vec<(&'static str, usize)> {
    include_str!("../line-cap-debt.txt")
        .lines()
        .filter(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| {
            let (name, n) = l.rsplit_once(' ')?;
            Some((name.trim(), n.trim().parse().ok()?))
        })
        .collect()
}

#[cfg(test)]
#[path = "linecap_tests.rs"]
mod tests;
