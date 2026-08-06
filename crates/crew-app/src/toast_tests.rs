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
    push_toasts(&mut scenes, &mut t, content, 8.0, 16.0, 2_000);
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
    push_toasts(&mut scenes, &mut t, content, 8.0, 16.0, 2_000);
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
    push_toasts(&mut scenes, &mut t, content, 8.0, 16.0, 1_000 + TTL_MS + 1);
    assert!(scenes.is_empty());
    assert!(t.items.is_empty(), "prune ran as part of the render pass");
}

#[test]
fn exit_window_fades_text_toward_the_page() {
    // fade = 1 exactly at expiry: the text cell fg equals page_bg.
    let cells_mid = card_cells("hi", "note", false, 6, 0.0);
    let cells_end = card_cells("hi", "note", false, 6, 1.0);
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
        card_cells("x", "waiting", alert, 8, 0.0)
            .iter()
            .find(|c| c.row == 0 && c.c == '─')
            .expect("border cell")
            .fg
    };
    assert_eq!(border_of(true), t.bell);
    assert_eq!(border_of(false), t.border_normal);
}

#[test]
fn long_text_clips_on_a_cell_boundary_with_ellipsis() {
    let clipped = clip_cols("abcdefgh", 5);
    assert_eq!(clipped, "abcd…");
    assert!(str_w(&clipped) <= 5);
    // Wide glyphs never split: clipping "日本語" at 5 keeps whole chars.
    let wide = clip_cols("日本語", 5);
    assert!(str_w(&wide) <= 5);
    assert!(wide.ends_with('…'));
}
