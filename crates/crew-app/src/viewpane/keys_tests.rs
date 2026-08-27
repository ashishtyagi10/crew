use super::*;
use winit::keyboard::{Key, NamedKey};

fn ch(s: &str) -> Key {
    Key::Character(s.into())
}

/// A `ViewPane` sitting on rendered content long enough to matter: 50 lines
/// against a 10-row viewport means real, un-clamped scroll movement is
/// observable, unlike the default `open()` fixture (whose `Loading` state
/// renders a single "loading…" line and so pins `clamp_scroll` to 0
/// regardless of what a scroll action did or didn't do).
fn pane_with(text: &str) -> crate::viewpane::ViewPane {
    use crate::viewpane::detect::Format;
    use crate::viewpane::load::Loaded;
    use crate::viewpane::LoadState;
    let mut p = crate::viewpane::ViewPane::open(std::env::temp_dir().join("k.txt"));
    p.state = LoadState::Ready {
        format: Format::Code { lang: "" },
        loaded: Loaded {
            text: text.into(),
            truncated: None,
            meta: None,
        },
    };
    p
}

/// `n` lines of content with no trailing newline, so the rendered line count
/// is exactly `n` — a trailing `"\n"` would `split('\n')` into an extra
/// empty final line and throw off the exact clamp-boundary math below.
fn lines(n: usize) -> String {
    (0..n)
        .map(|i| format!("line{i}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn releases_do_nothing() {
    assert_eq!(
        view_key(&Key::Named(NamedKey::Escape), false, false),
        ViewInput::Ignore
    );
}

#[test]
fn escape_closes() {
    assert_eq!(
        view_key(&Key::Named(NamedKey::Escape), true, false),
        ViewInput::Close
    );
}

#[test]
fn scroll_keys_map() {
    assert_eq!(
        view_key(&Key::Named(NamedKey::ArrowDown), true, false),
        ViewInput::Down
    );
    assert_eq!(
        view_key(&Key::Named(NamedKey::PageUp), true, false),
        ViewInput::PageUp
    );
    assert_eq!(
        view_key(&Key::Named(NamedKey::Home), true, false),
        ViewInput::Top
    );
    assert_eq!(
        view_key(&Key::Named(NamedKey::End), true, false),
        ViewInput::Bottom
    );
}

#[test]
fn letter_keys_map_and_ignore_case() {
    assert_eq!(view_key(&ch("e"), true, false), ViewInput::Edit);
    assert_eq!(view_key(&ch("O"), true, false), ViewInput::OpenExternal);
    assert_eq!(view_key(&ch("r"), true, false), ViewInput::Reload);
    assert_eq!(view_key(&ch("s"), true, false), ViewInput::ToggleRaw);
}

#[test]
fn a_ctrl_chord_never_reaches_a_letter_action() {
    // Defense in depth: Ctrl+R must not reload the file out from under a
    // chord the app handles globally.
    for k in ["e", "o", "r", "s"] {
        assert_eq!(view_key(&ch(k), true, true), ViewInput::Ignore);
    }
}

#[test]
fn toggling_raw_flips_the_flag_without_asking_the_app_to_do_anything() {
    let mut p = crate::viewpane::ViewPane::open(std::env::temp_dir().join("k.txt"));
    assert!(!p.raw);
    let act = apply(&mut p, ViewInput::ToggleRaw, 40, 10);
    assert!(p.raw, "s toggles");
    assert!(act.is_none(), "no host action needed");
}

#[test]
fn edit_and_open_carry_the_path() {
    let path = std::env::temp_dir().join("k.txt");
    let mut p = crate::viewpane::ViewPane::open(path.clone());
    match apply(&mut p, ViewInput::Edit, 40, 10) {
        Some(ViewAction::Edit(p2)) => assert_eq!(p2, path),
        _ => panic!("e produces Edit with the path"),
    }
    match apply(&mut p, ViewInput::OpenExternal, 40, 10) {
        Some(ViewAction::OpenExternal(p2)) => assert_eq!(p2, path),
        _ => panic!("o produces OpenExternal with the path"),
    }
}

#[test]
fn reload_action_is_returned_from_apply() {
    let mut p = crate::viewpane::ViewPane::open(std::env::temp_dir().join("k.txt"));
    assert!(matches!(
        apply(&mut p, ViewInput::Reload, 40, 10),
        Some(ViewAction::Reload)
    ));
}

// Scrolling up from a NONZERO offset, because the obvious version — scroll
// up from 0, assert still 0 — passes against a no-op `apply` that does
// nothing at all. Verified by injecting exactly that mutation.
#[test]
fn scroll_up_actually_decrements_from_a_nonzero_offset() {
    let mut p = pane_with(&lines(50));
    p.scroll = 5;
    apply(&mut p, ViewInput::Up, 40, 10);
    assert_eq!(p.scroll, 4);
}

#[test]
fn down_page_and_bounds_scroll_by_the_right_amounts() {
    let mut p = pane_with(&lines(50));
    apply(&mut p, ViewInput::Down, 40, 10);
    assert_eq!(p.scroll, 1);
    apply(&mut p, ViewInput::PageDown, 40, 10);
    assert_eq!(p.scroll, 1 + PAGE as usize);
    apply(&mut p, ViewInput::Up, 40, 10);
    assert_eq!(p.scroll, PAGE as usize);
    apply(&mut p, ViewInput::PageUp, 40, 10);
    assert_eq!(p.scroll, 0);
    apply(&mut p, ViewInput::Bottom, 40, 10);
    assert_eq!(p.scroll, 40, "clamped to the last full page (50 - 10 rows)");
    apply(&mut p, ViewInput::Top, 40, 10);
    assert_eq!(p.scroll, 0);
}

#[test]
fn scroll_wheel_treats_a_positive_delta_as_scrolling_up() {
    // A positive `lines` (documented "up/older" by `scroll::scroll_pane`,
    // the caller) must DECREASE the stored offset, toward the top —
    // `self.scroll` counts rows down from the top. Getting the sign
    // backwards here passes every other test in this file and is only
    // visible as scrolling the wrong direction in the running app.
    let mut p = pane_with(&lines(50));
    p.scroll = 10;
    p.scroll_wheel(40, 10, 3);
    assert_eq!(p.scroll, 7, "positive lines scrolls up, toward the top");
    p.scroll_wheel(40, 10, -3);
    assert_eq!(p.scroll, 10, "negative lines scrolls back down");
}

// --- search wiring (`/`, typing, `n`/`N`, Enter, two-stage Esc) ---

#[test]
fn escape_cancels_a_live_search_before_it_closes_the_pane() {
    let mut p = crate::viewpane::ViewPane::open(std::env::temp_dir().join("k.txt"));
    apply(&mut p, ViewInput::Slash, 40, 10);
    assert!(
        apply(&mut p, ViewInput::Close, 40, 10).is_none(),
        "first Esc cancels the search"
    );
    assert!(p.search.is_none());
    assert!(
        matches!(
            apply(&mut p, ViewInput::Close, 40, 10),
            Some(ViewAction::Close)
        ),
        "second Esc closes the pane"
    );
}

#[test]
fn slash_opens_an_empty_search_in_typing_mode() {
    let mut p = pane_with("alpha\nbeta\n");
    let act = apply(&mut p, ViewInput::Slash, 40, 10);
    assert!(act.is_none());
    let search = p.search.as_ref().expect("/ starts a search");
    assert!(search.typing);
    assert!(search.needle.is_empty());
    assert!(search.hits.is_empty());
}

#[test]
fn typing_characters_build_the_needle_and_recompute_hits() {
    let mut p = pane_with("alpha\nbeta\nalpha again\n");
    apply(&mut p, ViewInput::Slash, 40, 10);
    for c in "alpha".chars() {
        apply(&mut p, ViewInput::Char(c), 40, 10);
    }
    let search = p.search.as_ref().unwrap();
    assert_eq!(search.needle, "alpha");
    assert_eq!(search.hits, vec![0, 2]);
}

#[test]
fn backspace_removes_the_last_needle_character_and_recomputes_hits() {
    let mut p = pane_with("alpha\nbeta\n");
    apply(&mut p, ViewInput::Slash, 40, 10);
    for c in "alphaX".chars() {
        apply(&mut p, ViewInput::Char(c), 40, 10);
    }
    assert!(
        p.search.as_ref().unwrap().hits.is_empty(),
        "sanity: 'alphaX' matches nothing"
    );
    apply(&mut p, ViewInput::Backspace, 40, 10);
    let search = p.search.as_ref().unwrap();
    assert_eq!(search.needle, "alpha");
    assert_eq!(search.hits, vec![0]);
}

// While typing, `view_key` has already folded e/o/r/s/n/N// into their
// normal-mode `ViewInput` actions — `apply` must reinterpret every one of
// them as the literal character it came from, or you simply cannot type
// "reset" or "error" into a search needle without firing Reload/Edit, and
// each of those inputs must NOT also fire its normal action while typing.
#[test]
fn letters_that_are_normally_actions_become_needle_characters_while_typing() {
    let mut p = pane_with("alpha\nbeta\n");
    apply(&mut p, ViewInput::Slash, 40, 10);
    for input in [
        ViewInput::Edit,
        ViewInput::OpenExternal,
        ViewInput::Reload,
        ViewInput::ToggleRaw,
        ViewInput::NextHit,
        ViewInput::PrevHit,
        ViewInput::Slash,
    ] {
        let act = apply(&mut p, input, 40, 10);
        assert!(act.is_none(), "typing must not fire a host action");
    }
    assert!(!p.raw, "s must not have toggled raw while typing");
    assert_eq!(p.search.as_ref().unwrap().needle, "eorsnN/");
}

#[test]
fn enter_stops_typing_and_jumps_to_the_first_hit() {
    let mut ls: Vec<String> = (0..30).map(|i| format!("line{i}")).collect();
    ls[20] = "needle".into();
    let mut p = pane_with(&ls.join("\n"));
    apply(&mut p, ViewInput::Slash, 40, 10);
    for c in "needle".chars() {
        apply(&mut p, ViewInput::Char(c), 40, 10);
    }
    apply(&mut p, ViewInput::Enter, 40, 10);
    let search = p.search.as_ref().unwrap();
    assert!(!search.typing, "Enter stops typing");
    assert_eq!(p.scroll, 20, "jumped to the only hit");
}

#[test]
fn next_hit_and_prev_hit_navigate_and_wrap() {
    // 40 lines against a 10-row viewport (clamp cap 30) keeps both hits
    // comfortably inside the clamp-safe range, so this test is purely about
    // navigation/wrap, not about clamp_scroll's own last-page behaviour
    // (covered separately by `render_tests::clamp_scroll_pulls_a_wild_offset_back_to_the_last_page`).
    let mut ls: Vec<String> = (0..40).map(|i| format!("line{i}")).collect();
    ls[5] = "needle".into();
    ls[25] = "needle".into();
    let mut p = pane_with(&ls.join("\n"));
    apply(&mut p, ViewInput::Slash, 40, 10);
    for c in "needle".chars() {
        apply(&mut p, ViewInput::Char(c), 40, 10);
    }
    apply(&mut p, ViewInput::Enter, 40, 10);
    assert_eq!(p.scroll, 5, "Enter lands on the first hit");
    apply(&mut p, ViewInput::NextHit, 40, 10);
    assert_eq!(p.scroll, 25);
    apply(&mut p, ViewInput::NextHit, 40, 10);
    assert_eq!(p.scroll, 5, "n wraps forward");
    apply(&mut p, ViewInput::PrevHit, 40, 10);
    assert_eq!(p.scroll, 25, "N wraps backward");
}

// The clamp fix above (Enter's jump now calls `clamp_scroll`) is itself
// worth proving directly: a hit landing past the last full page must not
// strand `scroll` beyond content length.
#[test]
fn enter_clamps_a_hit_that_lands_past_the_last_page() {
    let mut ls: Vec<String> = (0..30).map(|i| format!("line{i}")).collect();
    ls[25] = "needle".into();
    let mut p = pane_with(&ls.join("\n"));
    apply(&mut p, ViewInput::Slash, 40, 10);
    for c in "needle".chars() {
        apply(&mut p, ViewInput::Char(c), 40, 10);
    }
    apply(&mut p, ViewInput::Enter, 40, 10);
    assert_eq!(
        p.scroll, 20,
        "clamped to the last full page (30 lines - 10 rows), not the raw hit line 25"
    );
}

#[test]
fn next_hit_without_an_active_search_does_nothing() {
    let mut p = pane_with(&lines(50));
    p.scroll = 3;
    let act = apply(&mut p, ViewInput::NextHit, 40, 10);
    assert!(act.is_none());
    assert_eq!(p.scroll, 3, "no search means nothing to jump to");
}

/// A markdown `ViewPane` whose rendered (non-raw) and raw layouts genuinely
/// differ in line-to-content shape: the raw rung adds a 6-column gutter and
/// wraps at a different width, and the rendered rung drops the `#` marker,
/// so a wrapped paragraph occupies a different number of rows in each mode.
/// Confirmed empirically (not just assumed) for this exact text at 20
/// columns: "needle" sits at rendered-line index 14 but raw-line index 16 —
/// exactly the shape needed to prove stale hit indexes land on the wrong
/// line after a raw toggle, not merely a bounds-safe one.
fn markdown_pane_with(text: &str) -> crate::viewpane::ViewPane {
    use crate::viewpane::detect::Format;
    use crate::viewpane::load::Loaded;
    use crate::viewpane::LoadState;
    let mut p = crate::viewpane::ViewPane::open(std::env::temp_dir().join("k.md"));
    p.state = LoadState::Ready {
        format: Format::Markdown,
        loaded: Loaded {
            text: text.into(),
            truncated: None,
            meta: None,
        },
    };
    p
}

const WRAP_SHIFTING_MARKDOWN: &str = "# Heading\n\nThis is a fairly long paragraph of body text that should wrap across several rows once it is laid out inside a narrow column width, forcing the renderer to break it into pieces.\n\nneedle here\n";

// Reachable by an ordinary sequence: `/needle` -> Enter -> `s` -> `n`.
// `hits` are line indexes into the OLD rendering; toggling `raw` re-renders
// under a different layout (see `markdown_pane_with`), so those indexes can
// point at completely different text. `jump` is bounds-safe (it clamps),
// which is exactly why this bug was silent: `n` never panics or goes out of
// range, it just quietly lands somewhere that isn't the needle.
#[test]
fn toggling_raw_recomputes_stale_hits_instead_of_landing_on_the_wrong_line() {
    let mut p = markdown_pane_with(WRAP_SHIFTING_MARKDOWN);
    let (cols, rows) = (20, 1);
    apply(&mut p, ViewInput::Slash, cols, rows);
    for c in "needle".chars() {
        apply(&mut p, ViewInput::Char(c), cols, rows);
    }
    apply(&mut p, ViewInput::Enter, cols, rows);
    apply(&mut p, ViewInput::ToggleRaw, cols, rows);
    apply(&mut p, ViewInput::NextHit, cols, rows);

    let cache = p.lines_for(cols);
    let line: String = cache.lines[p.scroll].iter().map(|c| c.c).collect();
    assert!(
        line.to_lowercase().contains("needle"),
        "landed on {line:?} at scroll {}, which does not contain the needle",
        p.scroll
    );
}

// Three occurrences of "needle", confirmed empirically (not assumed) to
// land at different indexes between the two layouts at cols=20:
// nonraw hits [2, 16, 27], raw hits [2, 18, 30]. A single-hit fixture can't
// tell "cursor reset" apart from "cursor left alone" — `next()` computes
// `(at + 1) % len`, and for `len == 1` that's the same value whether `at`
// starts at `Some(0)` or gets reset to `None`. Three hits, with the cursor
// walked to the SECOND one before the toggle, makes the two behaviors
// diverge: a reset cursor lands on the new list's first hit (raw index 2);
// a stale `Some(1)` instead computes `(1 + 1) % 3 = 2`, landing on the new
// list's THIRD hit (raw index 30) — still a real hit, still bounds-safe,
// still contains "needle", and still the wrong one.
const WRAP_SHIFTING_MARKDOWN_MULTI_HIT: &str = "# Heading\n\nneedle one\n\nThis is a fairly long paragraph of body text that should wrap across several rows once it is laid out inside a narrow column width, forcing the renderer to break it into pieces.\n\nneedle two\n\nAnother paragraph of filler text goes here to push things further down the page before the third occurrence shows up.\n\nneedle three\n";

#[test]
fn toggling_raw_resets_the_hit_cursor_not_just_the_hit_list() {
    let mut p = markdown_pane_with(WRAP_SHIFTING_MARKDOWN_MULTI_HIT);
    let (cols, rows) = (20, 1);
    apply(&mut p, ViewInput::Slash, cols, rows);
    for c in "needle".chars() {
        apply(&mut p, ViewInput::Char(c), cols, rows);
    }
    apply(&mut p, ViewInput::Enter, cols, rows); // at = Some(0): first hit
    apply(&mut p, ViewInput::NextHit, cols, rows); // at = Some(1): second hit
    apply(&mut p, ViewInput::ToggleRaw, cols, rows); // re-render; hits recomputed
    apply(&mut p, ViewInput::NextHit, cols, rows);

    assert_eq!(
        p.scroll, 2,
        "a reset cursor lands on the new list's FIRST hit (raw index 2); \
         a stale one lands on its third (raw index 30) instead"
    );
}

/// As [`pane_with`], for a rung the test names itself.
fn ready_pane(format: crate::viewpane::detect::Format, text: &str) -> crate::viewpane::ViewPane {
    use crate::viewpane::load::Loaded;
    use crate::viewpane::LoadState;
    let mut p = crate::viewpane::ViewPane::open(std::env::temp_dir().join("k.diff"));
    p.state = LoadState::Ready {
        format,
        loaded: Loaded {
            text: text.into(),
            truncated: None,
            meta: None,
        },
    };
    p.cache.replace(None);
    p
}

/// `]` and `[` walk a review: file header, hunk, hunk, next file. The rows
/// come from the RENDERED document, so this is also the check that a landmark
/// found in the source lands on the row it was rendered at.
#[test]
fn brackets_step_a_diff_file_by_file_and_hunk_by_hunk() {
    let _g = crate::app::theme_test_guard();
    // Trailing context so the last landmark is reachable: `clamp_scroll`
    // stops at the last page, and a mark inside it could never be landed on
    // exactly — which would be the clamp under test, not the jump.
    let tail = " ok\n".repeat(12);
    let text = format!("diff --git a/a.rs b/a.rs\n@@ -1 +1 @@ fn one\n-a\n+b\n@@ -9 +9 @@ fn two\n-c\n+d\ndiff --git a/b.rs b/b.rs\n@@ -1 +1 @@ fn three\n-e\n+f\n{tail}");
    let mut p = ready_pane(crate::viewpane::detect::Format::Diff, &text);
    let (cols, rows) = (60, 6);
    let rows_of = |p: &crate::viewpane::ViewPane| {
        p.lines_for(cols)
            .marks
            .iter()
            .map(|m| m.row)
            .collect::<Vec<_>>()
    };
    let marks = rows_of(&p);
    assert_eq!(marks.len(), 5, "five landmarks in this review");
    assert_eq!(p.scroll, 0);
    for want in &marks[1..] {
        apply(&mut p, ViewInput::NextMark, cols, rows);
        assert_eq!(p.scroll, *want, "] did not land on {want}");
    }
    // …and back up the same way.
    for want in marks[..marks.len() - 1].iter().rev() {
        apply(&mut p, ViewInput::PrevMark, cols, rows);
        assert_eq!(p.scroll, *want, "[ did not land on {want}");
    }
}

/// At the last landmark `]` does nothing rather than wrapping to the top —
/// losing your place in a long review is worse than a key that no-ops.
#[test]
fn stepping_past_the_last_landmark_stays_put() {
    let _g = crate::app::theme_test_guard();
    let mut p = ready_pane(
        crate::viewpane::detect::Format::Diff,
        "@@ -1 +1 @@ only\n-a\n+b\n",
    );
    let (cols, rows) = (60, 6);
    apply(&mut p, ViewInput::NextMark, cols, rows);
    let at = p.scroll;
    apply(&mut p, ViewInput::NextMark, cols, rows);
    assert_eq!(p.scroll, at);
    apply(&mut p, ViewInput::PrevMark, cols, rows);
    apply(&mut p, ViewInput::PrevMark, cols, rows);
    assert_eq!(p.scroll, 0);
}

/// A file with no structure has nothing to step through, and the keys leave
/// the view exactly where it was.
#[test]
fn brackets_do_nothing_in_a_document_with_no_landmarks() {
    let _g = crate::app::theme_test_guard();
    let mut p = ready_pane(
        crate::viewpane::detect::Format::Code { lang: "" },
        "one\ntwo\nthree\n",
    );
    let (cols, rows) = (60, 6);
    apply(&mut p, ViewInput::Down, cols, rows);
    let at = p.scroll;
    apply(&mut p, ViewInput::NextMark, cols, rows);
    apply(&mut p, ViewInput::PrevMark, cols, rows);
    assert_eq!(p.scroll, at);
}

/// A long document is walked by its headings, the same way a review is walked
/// by its hunks — and the renderer, which already decided which lines are
/// headings, is what says where they are.
#[test]
fn brackets_step_a_markdown_document_by_its_headings() {
    let _g = crate::app::theme_test_guard();
    let body = "text\n".repeat(6);
    let text = format!("# One\n\n{body}\n## Two\n\n{body}\n## Three\n\n{body}");
    let mut p = ready_pane(crate::viewpane::detect::Format::Markdown, &text);
    let (cols, rows) = (60, 6);
    let marks: Vec<(usize, String)> = p
        .lines_for(cols)
        .marks
        .iter()
        .map(|m| (m.row, m.label.clone()))
        .collect();
    let labels: Vec<&str> = marks.iter().map(|(_, l)| l.as_str()).collect();
    assert_eq!(labels, vec!["One", "Two", "Three"], "{marks:?}");
    apply(&mut p, ViewInput::NextMark, cols, rows);
    assert_eq!(p.scroll, marks[1].0);
    apply(&mut p, ViewInput::PrevMark, cols, rows);
    assert_eq!(p.scroll, marks[0].0);
}

/// Raw mode is the escape hatch for reading the bytes; there are no rendered
/// headings there, so there is nothing to step.
#[test]
fn the_raw_view_of_a_document_has_no_landmarks() {
    let _g = crate::app::theme_test_guard();
    let mut p = ready_pane(crate::viewpane::detect::Format::Markdown, "# One\n\nbody\n");
    p.raw = true;
    p.cache.replace(None);
    assert!(p.lines_for(60).marks.is_empty());
}
