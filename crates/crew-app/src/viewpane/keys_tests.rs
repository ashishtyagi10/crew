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
fn scroll_wheel_matches_mdpane_sign_convention_positive_is_up() {
    // `MdPane::scroll_wheel` routes into `MdPane::scroll`, which computes
    // `target - delta` — so a positive `lines` (documented "up/older" by
    // `scroll::scroll_pane`) DECREASES the stored offset. Confirmed by
    // reading `mdpane.rs`/`scroll.rs` before writing this: getting the sign
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
