use super::*;

fn toasts_with(n: usize, now: u64) -> Toasts {
    let mut t = Toasts::default();
    for i in 0..n {
        t.push(format!("toast {i}"), "note", false, now);
    }
    t
}

#[test]
fn push_caps_the_stack_dropping_oldest_first() {
    let mut t = toasts_with(MAX_SHOWN, 1_000);
    t.push("newest".into(), "note", false, 1_000);
    assert_eq!(t.items.len(), MAX_SHOWN);
    assert_eq!(t.items[0].text, "toast 1", "oldest was dropped");
    assert_eq!(t.items.last().unwrap().text, "newest");
}

#[test]
fn prune_removes_only_expired() {
    let mut t = toasts_with(1, 1_000);
    t.push("young".into(), "note", false, 1_000 + TTL_MS - 100);
    t.prune(1_000 + TTL_MS);
    assert_eq!(t.items.len(), 1);
    assert_eq!(t.items[0].text, "young");
}

/// Frames are wanted while sliding in and inside the exit window — NOT while
/// resting in between. That gap is what keeps a toast from repainting an idle
/// crew for its whole 4.8s life.
#[test]
fn any_live_covers_entry_and_exit_but_not_the_rest() {
    let t = toasts_with(1, 1_000);
    assert!(t.any_live(1_050), "sliding in");
    assert!(
        !t.any_live(1_000 + TTL_MS / 2),
        "resting mid-life must not want frames"
    );
    assert!(t.any_live(1_000 + TTL_MS - EXIT_MS + 10), "exit window");
    assert!(!t.any_live(1_000 + TTL_MS + 10), "expired");
}

#[test]
fn card_geometry_is_cell_quantized_and_right_aligned() {
    let mut t = toasts_with(1, 1_000);
    let mut scenes = Vec::new();
    let content = Rect {
        x: 100.0,
        y: 0.0,
        w: 800.0,
        h: 600.0,
    };
    // Far past the slide (still alive): resting position.
    push_toasts(&mut scenes, &mut t, content, 8.0, 16.0, 2_000, None);
    assert_eq!(scenes.len(), 1);
    let s = &scenes[0];
    // "toast 0" = 7 cols + 4 frame/pad = 11 cols → 88px wide, 3 rows tall.
    assert_eq!((s.w, s.h), (88.0, 48.0));
    // Right-aligned to content minus the gap: 100 + 800 - 8 - 88.
    assert_eq!(s.x, 804.0);
    assert!(s.overlay, "toasts must ride the opaque overlay pass");
    assert!(!s.glass);
}

#[test]
fn cards_stack_downward_with_a_gap() {
    let mut t = toasts_with(2, 1_000);
    let mut scenes = Vec::new();
    let content = Rect {
        x: 0.0,
        y: 0.0,
        w: 800.0,
        h: 600.0,
    };
    push_toasts(&mut scenes, &mut t, content, 8.0, 16.0, 2_000, None);
    assert_eq!(scenes.len(), 2);
    assert_eq!(scenes[0].y, 8.0);
    assert_eq!(scenes[1].y, 8.0 + 48.0 + 8.0);
}

#[test]
fn expired_toasts_render_nothing() {
    let mut t = toasts_with(3, 1_000);
    let mut scenes = Vec::new();
    let content = Rect {
        x: 0.0,
        y: 0.0,
        w: 800.0,
        h: 600.0,
    };
    push_toasts(
        &mut scenes,
        &mut t,
        content,
        8.0,
        16.0,
        1_000 + TTL_MS + 1,
        None,
    );
    assert!(scenes.is_empty());
    assert!(t.items.is_empty(), "prune ran as part of the render pass");
}

#[test]
fn exit_window_fades_text_toward_the_page() {
    // fade = 1 exactly at expiry: the text cell fg equals page_bg.
    let cells_mid = card_cells(
        &CardText {
            text: "hi",
            legend: "note",
            repeats: 1,
            alert: false,
            actionable: false,
        },
        6,
        0.0,
        false,
    );
    let cells_end = card_cells(
        &CardText {
            text: "hi",
            legend: "note",
            repeats: 1,
            alert: false,
            actionable: false,
        },
        6,
        1.0,
        false,
    );
    let t = crew_theme::theme();
    let text_cell = |cells: &Vec<crew_render::CellView>| {
        cells
            .iter()
            .find(|c| c.row == 1 && c.c == 'h')
            .expect("text cell")
            .fg
    };
    assert_eq!(text_cell(&cells_mid), t.ink);
    assert_eq!(text_cell(&cells_end), t.page_bg);
}

#[test]
fn alert_toasts_border_in_the_bell_color() {
    let t = crew_theme::theme();
    let border_of = |alert: bool| {
        card_cells(
            &CardText {
                text: "x",
                legend: "waiting",
                repeats: 1,
                alert,
                actionable: false,
            },
            8,
            0.0,
            false,
        )
        .iter()
        .find(|c| c.row == 0 && c.c == '─')
        .expect("border cell")
        .fg
    };
    // The alert stroke is EXACTLY the bell — the gradient does not get to
    // repaint a warning (see `card_cells`).
    assert_eq!(border_of(true), t.bell);
    // The ordinary toast carries the quiet gradient, so it is no longer the
    // flat border colour — but it is still lit at that colour's brightness,
    // which is what keeps a toast from out-shouting the focused pane.
    let quiet = border_of(false);
    assert_ne!(quiet, t.border_normal, "an ordinary toast should be tinted");
    let luma = |c: (u8, u8, u8)| {
        0.2126 * f32::from(c.0) + 0.7152 * f32::from(c.1) + 0.0722 * f32::from(c.2)
    };
    let (want, got) = (luma(t.border_normal), luma(quiet));
    assert!(
        (got - want).abs() <= 2.0,
        "tinted toast border {quiet:?} (luma {got:.1}) drifted from {:?} (luma {want:.1})",
        t.border_normal
    );
}

#[test]
fn long_text_clips_on_a_cell_boundary_with_ellipsis() {
    // The clip itself lives in `chatwidth::clip_w` (shared with every card
    // legend); here we assert the toast body actually goes through it.
    let mut t = Toasts::default();
    t.push("x".repeat(MAX_TEXT_COLS + 20), "note", false, 1_000);
    let mut scenes = Vec::new();
    let content = Rect {
        x: 0.0,
        y: 0.0,
        w: 800.0,
        h: 600.0,
    };
    push_toasts(&mut scenes, &mut t, content, 8.0, 16.0, 2_000, None);
    let s = &scenes[0];
    assert_eq!(s.w, (MAX_TEXT_COLS + 4) as f32 * 8.0, "card caps its width");
    assert!(
        s.cells.iter().any(|c| c.row == 1 && c.c == '…'),
        "over-wide toast text must end in an ellipsis"
    );
}

/// The rect a card is hit-tested against must be the rect the frame drew,
/// slide offsets and all. Recording it at layout time is the only way that
/// holds: a re-derived rect drifts the moment the stacking or the easing
/// changes, and the symptom is a click that misses a card by a few pixels.
#[test]
fn the_hit_rect_is_the_rect_the_frame_drew() {
    let mut t = Toasts::default();
    t.push("hello".into(), "note", false, 0);
    let mut scenes = Vec::new();
    let content = Rect {
        x: 0.0,
        y: 0.0,
        w: 800.0,
        h: 600.0,
    };
    // Well past the slide so the card is at rest and the scene's own x/y are
    // the settled ones.
    push_toasts(&mut scenes, &mut t, content, 8.0, 16.0, 1_000, None);
    let s = &scenes[0];
    assert_eq!(t.index_at(s.x + 1.0, s.y + 1.0), Some(0));
    assert_eq!(t.index_at(s.x - 1.0, s.y + 1.0), None, "left of the card");
    assert_eq!(
        t.index_at(s.x + 1.0, s.y + s.h + 1.0),
        None,
        "below the card"
    );
}

/// WCAG's Pause, Stop, Hide: content that auto-hides on a timer has to be
/// pausable. The failure this guards is the one that makes the feature
/// pointless — a hold that reads the pointer but never actually stops the
/// clock, so the card still vanishes out from under the reader.
#[test]
fn resting_the_pointer_holds_the_whole_stack_past_its_ttl() {
    let mut t = Toasts::default();
    t.push("first".into(), "note", false, 0);
    t.push("second".into(), "note", false, 0);
    let mut scenes = Vec::new();
    let content = Rect {
        x: 0.0,
        y: 0.0,
        w: 800.0,
        h: 600.0,
    };
    push_toasts(&mut scenes, &mut t, content, 8.0, 16.0, 100, None);
    let inside = (scenes[0].x + 2.0, scenes[0].y + 2.0);

    // Walk well past the TTL with the pointer resting on the top card.
    let mut now = 100;
    while now < TTL_MS * 3 {
        now += 100;
        scenes.clear();
        push_toasts(&mut scenes, &mut t, content, 8.0, 16.0, now, Some(inside));
    }
    assert_eq!(scenes.len(), 2, "a held stack must not expire");
    assert!(
        !t.any_live(now),
        "a held stack is frozen, so it must not ask for frames"
    );

    // Pointer leaves: the clock runs again and both cards go.
    now += TTL_MS + 1;
    scenes.clear();
    push_toasts(&mut scenes, &mut t, content, 8.0, 16.0, now, None);
    assert!(scenes.is_empty(), "the hold must not outlive the pointer");
}

/// A card that names a pane is a shortcut to it; one that names none is still
/// dismissible. Both leave the stack when clicked — a card that has been
/// answered and stays on screen says it wasn't.
#[test]
fn a_click_takes_the_card_and_reports_its_pane() {
    let mut t = Toasts::default();
    t.push_for(
        "agent-7 is waiting".into(),
        "waiting",
        true,
        0,
        Some("agent-7".into()),
    );
    t.push("copied".into(), "note", false, 0);
    let mut scenes = Vec::new();
    let content = Rect {
        x: 0.0,
        y: 0.0,
        w: 800.0,
        h: 600.0,
    };
    push_toasts(&mut scenes, &mut t, content, 8.0, 16.0, 1_000, None);
    let at = |i: usize| (scenes[i].x + 2.0, scenes[i].y + 2.0);

    let (x, y) = at(1);
    assert_eq!(t.pane_at(x, y), None, "a plain note names no pane");
    assert!(t.dismiss_at(x, y));

    let (x, y) = at(0);
    assert_eq!(t.pane_at(x, y), Some("agent-7"));
    assert!(t.dismiss_at(x, y));
    assert!(
        !t.dismiss_at(x, y),
        "the card is gone; the click falls through"
    );
}

/// The same thing said twice is one thing that happened twice: the card
/// counts up in place instead of the stack filling with copies.
#[test]
fn a_repeated_notification_counts_up_instead_of_stacking() {
    let mut t = Toasts::default();
    for _ in 0..3 {
        t.push_for(
            "agent-7 is waiting".into(),
            "waiting",
            true,
            1_000,
            Some("agent-7".into()),
        );
    }
    assert_eq!(t.items.len(), 1, "one event, one card");
    assert_eq!(t.items[0].repeats, 3);
}

/// The match is on everything the card says — text, legend and the pane it
/// would open — so two different events never merge into one wrong count.
#[test]
fn cards_that_say_different_things_never_merge() {
    let mut t = Toasts::default();
    t.push_for("done".into(), "done", false, 1_000, Some("a".into()));
    t.push_for("done".into(), "done", false, 1_000, Some("b".into()));
    t.push_for("done".into(), "bell", false, 1_000, Some("a".into()));
    t.push_for("finished".into(), "done", false, 1_000, Some("a".into()));
    assert_eq!(t.items.len(), 4);
    assert!(t.items.iter().all(|x| x.repeats == 1));
}

/// A repeat restarts the card's life WHERE IT IS. Promoting it to the bottom
/// of the stack would slide every other card, and the pointer may be resting
/// on one of them.
#[test]
fn a_repeat_restarts_the_card_in_place() {
    let mut t = Toasts::default();
    t.push("first".into(), "note", false, 0);
    t.push("second".into(), "note", false, 0);
    // Nearly expired…
    t.push("first".into(), "note", false, TTL_MS - 1);
    assert_eq!(t.items.len(), 2);
    assert_eq!(t.items[0].text, "first", "it did not move");
    t.prune(TTL_MS);
    assert_eq!(t.items.len(), 1, "the untouched card expired");
    assert_eq!(t.items[0].text, "first", "and the repeated one lives on");
}

/// A card that repeats does not evict its neighbours to say so — which is
/// what made this worth fixing: a pattern matching every line of output could
/// wipe the stack of every other notification crew had.
#[test]
fn a_flood_of_one_event_no_longer_clears_the_stack() {
    let mut t = toasts_with(MAX_SHOWN - 1, 1_000);
    for _ in 0..20 {
        t.push("noisy".into(), "note", false, 1_000);
    }
    assert_eq!(t.items.len(), MAX_SHOWN);
    assert_eq!(t.items[0].text, "toast 0", "the oldest card survived");
    assert_eq!(t.items.last().unwrap().repeats, 20);
}

/// The count is on the legend, and it survives the hover rewrite — the
/// reason you are hovering may well be that the card said it happened four
/// times.
#[test]
fn the_count_is_drawn_on_the_legend_and_survives_hover() {
    let _g = crate::app::theme_test_guard();
    let text_of = |cells: Vec<CellView>| -> String {
        let mut v: Vec<&CellView> = cells.iter().filter(|c| c.row == 0).collect();
        v.sort_by_key(|c| c.col);
        v.iter().map(|c| c.c).collect()
    };
    let once = text_of(card_cells(
        &CardText {
            text: "x",
            legend: "done",
            repeats: 1,
            alert: false,
            actionable: true,
        },
        30,
        0.0,
        false,
    ));
    assert!(
        !once.contains('\u{d7}'),
        "a first arrival counts nothing: {once:?}"
    );
    let four = text_of(card_cells(
        &CardText {
            text: "x",
            legend: "done",
            repeats: 4,
            alert: false,
            actionable: true,
        },
        30,
        0.0,
        false,
    ));
    assert!(four.contains("done \u{d7}4"), "legend was {four:?}");
    let hovered = text_of(card_cells(
        &CardText {
            text: "x",
            legend: "done",
            repeats: 4,
            alert: false,
            actionable: true,
        },
        30,
        0.0,
        true,
    ));
    assert!(
        hovered.contains("done \u{d7}4"),
        "hover kept the count: {hovered:?}"
    );
    assert!(hovered.contains("open"), "and still offers the click");
}
