//! `/keys` holds every pane kind to its own key map: the tables in
//! [`crate::helppanes`] are read against the source of each pane's `keys.rs`.
use crate::helppanes::{
    DISK_BINDINGS, DOC_BINDINGS, FAR_BINDINGS, SETTINGS_BINDINGS, TODO_BINDINGS, VIEW_BINDINGS,
};

/// The keys a source file's key map answers to, as the overlay would have to
/// spell them: single characters from `"x" =>` arms, and the function keys
/// from `NamedKey::F<n>`.
fn keys_in(src: &str) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    // `"k" => Some(Edit::…)` is how a document window spells its chords;
    // `.as_str() == "k"` is how the viewer AND the disk map spell a letter
    // (the viewer binds it as `s`, the map as `c` — the pattern must not
    // care which, or the map's `r` is invisible to this test, as it was).
    for (pat, take) in [
        (".as_str() == \"", 1),
        ("\" => ViewInput::", 0),
        ("\" => Some(Edit::", 0),
    ] {
        for (i, _) in src.match_indices(pat) {
            let key = match take {
                1 => src[i + pat.len()..].chars().next(),
                _ => src[..i].chars().last(),
            };
            if let Some(k) = key.filter(|c| !c.is_whitespace()) {
                keys.push(k.to_string());
            }
        }
    }
    // The other spelling: `Char('d')` and `Some('a')` arms, which is how the
    // todo pane and the settings form name their letters.
    for pat in ["Char('", "Some('"] {
        for (i, _) in src.match_indices(pat) {
            let rest = &src[i + pat.len()..];
            let mut cs = rest.chars();
            if let (Some(k), Some('\'')) = (cs.next(), cs.next()) {
                if !k.is_whitespace() {
                    keys.push(k.to_string());
                }
            }
        }
    }
    for (i, _) in src.match_indices("NamedKey::F") {
        let digits: String = src[i + "NamedKey::F".len()..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        if !digits.is_empty() {
            keys.push(format!("F{digits}"));
        }
    }
    keys.sort();
    keys.dedup();
    keys
}

/// The individual keys a binding table names, out of its KEY column only.
///
/// Matching against the descriptions too let `v` be "found" inside the word
/// *viewer*, so deleting its row changed nothing — the difference between a
/// parity test and a test that always passes. And the split runs on the
/// separators BEFORE `/`, or the search key (a row spelled `/ · n / N`) is
/// split out of existence by the very character it is named after.
fn listed_in(table: &[(&str, &str)]) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    for (k, _) in table {
        for tok in k
            .split(['\u{b7}', ' '])
            .map(str::trim)
            .filter(|t| !t.is_empty())
        {
            out.insert(tok.to_string());
            for part in tok.split('/').filter(|p| !p.is_empty()) {
                out.insert(part.to_string());
                // …and the key a modifier chord ends in: `F2` is reachable
                // only as `Alt+F2`, and the key map only knows it as `F2`.
                // Lowercased too, because the table writes a chord's letter
                // as `Ctrl+A` and the key map matches the character `'a'` —
                // the same key, spelled the way each side spells keys. The
                // viewer's `n`/`N` are a genuine pair and are listed as one.
                for bare in [part, part.rsplit('+').next().unwrap_or(part)] {
                    out.insert(bare.to_string());
                    out.insert(bare.to_lowercase());
                }
            }
        }
    }
    out
}

/// Every key a pane kind answers to is in the overlay.
///
/// Read out of the key map itself rather than listed a second time here: two
/// lists with nothing comparing them is exactly how `Ctrl+O` came to be
/// implemented, tested, and in neither list — and how the file viewer's keys
/// and the `/far` panel's function-key row came to be written down in the
/// manual and nowhere a user could find them without reading it.
#[test]
fn every_pane_key_is_in_the_overlay() {
    let panes = [
        (
            "the file viewer",
            include_str!("viewpane/keys.rs"),
            VIEW_BINDINGS,
            8,
        ),
        (
            "a /far panel",
            include_str!("farpane/keys.rs"),
            FAR_BINDINGS,
            8,
        ),
        (
            "the /todo pane",
            include_str!("todopane/keys.rs"),
            TODO_BINDINGS,
            8,
        ),
        (
            "a document window",
            include_str!("docwin/keys.rs"),
            DOC_BINDINGS,
            8,
        ),
        // The map's arrows are named keys; `r` is the letter it has.
        (
            "the /disk map",
            include_str!("diskpane.rs"),
            DISK_BINDINGS,
            1,
        ),
        // The settings form names no letters of its own — every field is
        // reached with named keys — so it has no floor to meet.
        (
            "/settings",
            include_str!("settingspane/keys.rs"),
            SETTINGS_BINDINGS,
            0,
        ),
    ];
    for (what, src, table, least) in panes {
        let keys = keys_in(src);
        assert!(keys.len() >= least, "{what}: the parse found only {keys:?}");
        let listed = listed_in(table);
        for k in keys {
            assert!(
                listed.contains(&k),
                "{what} answers to `{k}` and /keys never says so \u{2014} {listed:?}"
            );
        }
    }
}
