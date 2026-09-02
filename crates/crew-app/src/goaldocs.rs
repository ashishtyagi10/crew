//! The goal documents are held to the tree, the way the changelog is.
//!
//! `docs/superpowers/goals/` is where a goal is set and where its status is
//! written when it ships; `2026-09-01-close-the-open-goals.md` asks, as its
//! last done-means, that no goal document claim a status the tree
//! contradicts. A status is prose and no test reads prose — but every claim
//! rests on two kinds of fact a test CAN read: the versions a document says
//! something shipped in, and the source files it points at. A cited version
//! with no changelog entry, or a cited file that no longer exists, is a
//! document drifting from the tree, and three of fourteen had drifted when
//! this was written (a renamed module, and two status lines a release behind).
//!
//! Test-only, like [`crate::linecap`]: nothing here runs in the app.

/// Every goal document, as `(file name, text)`.
#[cfg(test)]
pub(crate) fn goal_docs() -> Vec<(String, String)> {
    let dir = root().join("docs/superpowers/goals");
    let mut docs: Vec<(String, String)> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            (name, std::fs::read_to_string(&p).unwrap())
        })
        .collect();
    docs.sort();
    docs
}

/// The repository root: this crate is `crates/crew-app`.
#[cfg(test)]
pub(crate) fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// Every `vX.Y.Z` in `text`, in order, with the `v` dropped.
#[cfg(test)]
pub(crate) fn cited_versions(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (i, _) in text.match_indices('v') {
        let rest = &text[i + 1..];
        let v: String = rest
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        // `v0.20.5.` at the end of a sentence: the full stop is prose.
        let v = v.trim_end_matches('.').to_string();
        let parts: Vec<&str> = v.split('.').collect();
        let word_before = text[..i].chars().last().is_some_and(char::is_alphanumeric);
        if !word_before && parts.len() == 3 && parts.iter().all(|p| !p.is_empty()) {
            out.push(v);
        }
    }
    out
}

/// Every backticked `.rs` path in `text` — `` `broker/session.rs:255` `` cites
/// `broker/session.rs`; a bare `` `keys.rs` `` cites any file of that name.
#[cfg(test)]
pub(crate) fn cited_sources(text: &str) -> Vec<String> {
    text.split('`')
        .skip(1)
        .step_by(2)
        .map(|span| span.split(':').next().unwrap_or(span))
        // `.rs` on its own is the extension being talked about, not a file.
        .filter(|p| p.ends_with(".rs") && !p.rsplit('/').next().unwrap_or(p).starts_with('.'))
        .filter(|p| {
            p.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | '-'))
        })
        .map(str::to_string)
        .collect()
}

/// Every `.rs` file under `crates/` and `vendor/`, as a path relative to the
/// root — the set a citation has to land in.
#[cfg(test)]
pub(crate) fn source_files() -> Vec<String> {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n != "target") {
                    walk(&p, out);
                }
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    let root = root();
    let mut found = Vec::new();
    for top in ["crates", "vendor"] {
        walk(&root.join(top), &mut found);
    }
    found
        .iter()
        .map(|p| {
            p.strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect()
}

/// Whether a citation names a file in `files`: the whole path, or a tail of
/// one at a `/` boundary.
#[cfg(test)]
pub(crate) fn resolves(cited: &str, files: &[String]) -> bool {
    files
        .iter()
        .any(|f| f == cited || f.ends_with(&format!("/{cited}")))
}

#[cfg(test)]
#[path = "goaldocs_tests.rs"]
mod tests;
