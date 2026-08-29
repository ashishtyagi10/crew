use super::*;

/// The overlay rebuilt row by row, which is how it is actually read.
/// `to_cells_opaque` fills every cell — blanks included — so scanning a
/// flat character stream matches text that never appears on one line.
fn rows_of(cells: &[CellView]) -> Vec<String> {
    let mut rows: std::collections::BTreeMap<u16, String> = Default::default();
    for c in cells {
        rows.entry(c.row).or_default().push(c.c);
    }
    rows.into_values().collect()
}

/// Letters and digits only, on both sides. The buffer fills blank cells
/// with a glyph that is neither a space nor whitespace, so neither a raw
/// nor a whitespace-squeezed comparison finds text that spans words.
fn letters(s: &str) -> String {
    s.chars().filter(char::is_ascii_alphanumeric).collect()
}

fn shows(cells: &[CellView], needle: &str) -> bool {
    let want = letters(needle);
    rows_of(cells).iter().any(|r| letters(r).contains(&want))
}

#[test]
fn renders_bindings_with_border() {
    let (w, h) = size();
    let cells = help_cells(w, h, 0, "");
    assert!(cells.iter().any(|c| c.c == '╭'));
    assert!(shows(&cells, "Ctrl+Tab"), "app bindings listed");
    assert!(shows(&cells, "in an agent pane"), "chat section listed");
}

/// The keys added with the plan card must be findable. A binding nobody
/// can discover is a binding nobody has.
#[test]
fn the_chat_pane_keys_are_documented() {
    let (w, h) = size();
    let cells = help_cells(w, h, 0, "");
    for needle in [
        "Enter",
        "Esc",
        "pending plan",
        "Shift+Enter",
        "Tab",
        "Recall a prompt",
        "Attach just those lines",
    ] {
        assert!(shows(&cells, needle), "missing {needle}");
    }
}

/// The overlay used to have to FIT a normal window: it did not scroll, so
/// a list one row too tall was cut off in silence and this test failed the
/// build until someone made room. Three times in one release "making room"
/// meant merging two unrelated rows and losing detail from both.
///
/// It scrolls now, so the contract is the one that actually matters: every
/// row is REACHABLE. The width still has to fit — nothing scrolls
/// sideways.
#[test]
fn every_binding_is_reachable_even_when_the_list_outgrows_the_window() {
    let (w, _) = size();
    assert!(w <= 130, "overlay is {w} cols and will be truncated");
    // A window shorter than the list, which is now the normal case.
    let h = 24u16;
    assert!(
        max_scroll(h, w, "") > 0,
        "premise: {} rows do not fit in {h}",
        crate::helplayout::logical().len()
    );
    let last = *crate::helplayout::logical()
        .last()
        .expect("a non-empty list");
    let cells = help_cells(w, h, max_scroll(h, w, ""), "");
    assert!(
        shows(&cells, last.1),
        "the last row ({:?}) is unreachable at max scroll",
        last.1
    );
    // …and the first row is still there before you scroll.
    let first = BINDINGS[0];
    assert!(shows(&help_cells(w, h, 0, ""), first.1), "the first row");
}

/// Scrolling stops where the list does. Running past the end into blank
/// space is what makes an unfamiliar scroll feel broken.
#[test]
fn scrolling_stops_at_the_end_of_the_list() {
    let (w, h) = (size().0, 24u16);
    let at_end = rows_of(&help_cells(w, h, max_scroll(h, w, ""), ""));
    assert_eq!(
        rows_of(&help_cells(w, h, max_scroll(h, w, "") + 50, "")),
        at_end,
        "scrolling past the end must draw the same thing"
    );
    let last = *crate::helplayout::logical()
        .last()
        .expect("a non-empty list");
    assert!(shows(&help_cells(w, h, max_scroll(h, w, ""), ""), last.1));
}

/// A scrollable thing that never says so is one nobody scrolls.
#[test]
fn the_overlay_says_when_there_is_more_below() {
    let (w, h) = (size().0, 24u16);
    assert!(rows_of(&help_cells(w, h, 0, ""))
        .join("")
        .contains('\u{2193}'));
    let end = rows_of(&help_cells(w, h, max_scroll(h, w, ""), "")).join("");
    assert!(!end.contains('\u{2193}'), "nothing more below at the end");
    assert!(end.contains('\u{2191}'), "but there is more above");
}

/// EVERY description in full, not just the ones that happened to fit.
/// `the_chat_pane_keys_are_documented` passed throughout the eight rows
/// that were being clipped, because each of its needles sat inside the
/// first 30 columns — an assertion on a prefix cannot see a missing tail.
#[test]
fn no_description_is_clipped() {
    let (w, h) = size();
    let cells = help_cells(w, h, 0, "");
    for (k, d) in BINDINGS.iter().chain(CHAT_BINDINGS) {
        assert!(shows(&cells, d), "clipped description for {k}: {d}");
    }
}

#[test]
fn tiny_renders_nothing() {
    assert!(help_cells(8, 3, 0, "").is_empty());
}

/// Chords the manual carries that the overlay deliberately does not: pane
/// cycling has a documented second spelling, and the overlay lists the
/// primary one. Declared, so a real omission stays visible.
const OVERLAY_OMITS: &[&str] = &["Cmd+]", "Cmd+["];

/// The manual says `/keys` shows "this list in-app". It did not:
/// **Ctrl+Shift+L** and **Ctrl+Shift+M** were documented and missing from
/// the overlay, and **Ctrl+O** was implemented, tested, and in neither list
/// — a working binding no user could discover from anywhere. Two lists,
/// nothing comparing them, exactly as before.
#[test]
fn the_overlay_and_the_manual_list_the_same_chords() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/CREW.md");
    let Ok(docs) = std::fs::read_to_string(&path) else {
        return; // docs not shipped in this build context
    };
    for (keys, _) in BINDINGS.iter().chain(CHAT_BINDINGS) {
        for chord in keys.split(" / ") {
            let chord = chord.trim();
            // Only modifier chords. Bare keys (`Enter`, `Right`, `Tab`) are
            // the composer's own editing verbs, and composer syntax
            // (`@a+b`) merely contains a `+` — a chord is where drift
            // hides, and a chord starts with a modifier.
            if !is_chord(chord) {
                continue;
            }
            assert!(
                docs.contains(chord),
                "the overlay shows `{chord}`, which docs/CREW.md never mentions"
            );
        }
    }
    for chord in documented_chords(&docs) {
        let listed = BINDINGS
            .iter()
            .chain(CHAT_BINDINGS)
            .any(|(k, _)| k.contains(&chord));
        assert!(
            listed || OVERLAY_OMITS.contains(&chord.as_str()),
            "docs/CREW.md documents `{chord}`, which /keys never shows"
        );
    }
}

/// A modifier chord, as opposed to a bare key or composer syntax.
fn is_chord(s: &str) -> bool {
    ["Cmd+", "Ctrl+", "Alt+", "Shift+"]
        .iter()
        .any(|m| s.starts_with(m))
}

/// The bolded chords in the manual's shortcuts table.
fn documented_chords(docs: &str) -> Vec<String> {
    let Some(start) = docs.find("## Keyboard shortcuts") else {
        return Vec::new();
    };
    let table = &docs[start..];
    let end = table[3..]
        .find("\n## ")
        .map(|i| i + 3)
        .unwrap_or(table.len());
    let mut out = Vec::new();
    let mut rest = &table[..end];
    while let Some(i) = rest.find("**") {
        rest = &rest[i + 2..];
        let Some(j) = rest.find("**") else { break };
        let span = rest[..j].trim().to_string();
        rest = &rest[j + 2..];
        if is_chord(&span) {
            out.push(span);
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Forty-odd bindings is a document, and the fastest way through a document
/// is to say what you are looking for.
#[test]
fn typing_filters_the_list_to_what_matches() {
    let _g = crate::app::theme_test_guard();
    let (w, h) = (super::size().0, 24);
    // `rows_of` stands blank cells in as `█`; the words are what matter here.
    let text = |needle: &str| -> String {
        rows_of(&help_cells(w, h, 0, needle))
            .join("\n")
            .replace('\u{2588}', " ")
    };
    let all = text("");
    let zoom = text("zoom");
    assert!(zoom.contains("zoom"), "{zoom}");
    assert!(
        !zoom.contains("Broadcast"),
        "an unmatched row survived:\n{zoom}"
    );
    assert!(all.contains("Broadcast"), "the fixture never had that row");
    // The window is a fixed height, so what shrinks is the number of rows
    // with anything written on them.
    let written = |s: &str| {
        s.lines()
            .filter(|l| {
                l.chars()
                    .filter(|c| !" \u{2502}\u{256d}\u{256e}\u{2570}\u{256f}\u{2500}".contains(*c))
                    .count()
                    > 1
            })
            .count()
    };
    assert!(
        written(&zoom) < written(&all),
        "{} rows of {} survived a one-word filter",
        written(&zoom),
        written(&all)
    );
}

/// Both halves of a row are searchable: the keys as well as the description.
#[test]
fn a_search_matches_the_chord_as_well_as_the_words() {
    let _g = crate::app::theme_test_guard();
    let (w, h) = (super::size().0, 24);
    let by_key = rows_of(&help_cells(w, h, 0, "ctrl+tab"))
        .join("\n")
        .replace('\u{2588}', " ");
    assert!(by_key.to_lowercase().contains("ctrl+tab"), "{by_key}");
}

/// A search matching nothing says so — an empty panel reads as a fault.
#[test]
fn a_search_with_no_match_says_so_rather_than_emptying() {
    let _g = crate::app::theme_test_guard();
    let (w, h) = (super::size().0, 24);
    let text = rows_of(&help_cells(w, h, 0, "zzzznope"))
        .join("\n")
        .replace('\u{2588}', " ");
    assert!(text.contains("no binding matches"), "{text}");
}

/// A heading is only true while something sits under it.
#[test]
fn a_section_heading_survives_only_with_its_rows() {
    let _g = crate::app::theme_test_guard();
    let (w, h) = (super::size().0, 24);
    let chat = rows_of(&help_cells(w, h, 0, "reverse-search"))
        .join("\n")
        .replace('\u{2588}', " ");
    assert!(chat.contains("in an agent pane"), "{chat}");
    let global = rows_of(&help_cells(w, h, 0, "zoom"))
        .join("\n")
        .replace('\u{2588}', " ");
    assert!(
        !global.contains("in an agent pane"),
        "a heading survived with no rows under it:\n{global}"
    );
}

/// The typed filter is shown where the version was — a filter you cannot see
/// is a list that looks broken — and the hint says the keys are live.
#[test]
fn the_overlay_shows_what_was_typed_and_offers_the_filter() {
    let _g = crate::app::theme_test_guard();
    let (w, h) = (super::size().0, 24);
    let all = rows_of(&help_cells(w, h, 0, ""))
        .join("\n")
        .replace('\u{2588}', " ");
    assert!(all.contains("type to filter"), "{all}");
    let filtered = rows_of(&help_cells(w, h, 0, "zoom"))
        .join("\n")
        .replace('\u{2588}', " ");
    assert!(filtered.contains("keys \u{b7} zoom"), "{filtered}");
}

/// The scroll limit follows the filtered list, or a narrowed overlay could be
/// scrolled far past its own end.
#[test]
fn the_scroll_limit_follows_the_filter() {
    let (w, _h) = (super::size().0, 24);
    let h = 12u16;
    let _ = w;
    assert!(max_scroll(h, w, "") > max_scroll(h, w, "zoom"));
    assert_eq!(max_scroll(h, w, "zzzznope"), 0);
}

/// The keys a source file's key map answers to, as the overlay would have to
/// spell them: single characters from `"x" =>` arms, and the function keys
/// from `NamedKey::F<n>`.
fn keys_in(src: &str) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    for (pat, take) in [("s.as_str() == \"", 1), ("\" => ViewInput::", 0)] {
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
