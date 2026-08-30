//! Off-screen render of the todo pane at the sizes the auto-grid hands a tile
//! and in the themes the app can be wearing.
//!
//! `/todo` is the one whole pane in the app with no pixel coverage at all —
//! every other surface (chat, Far, the drawn panes, the sidebar, help, the
//! menus) has a shot harness, and every one of those sweeps found something.
//! The todo list is exactly the shape that goes wrong off its design size: a
//! row with a title on the left and a `@project` plus a due stamp pushed
//! right, a composer card at the bottom, and a popup that opens over the list.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `cargo test -p crew-app --bin crew todo_shot -- --ignored --nocapture`
use crate::shotgpu_tests::{ink, shot_at};
use crate::todopane::{item::TodoItem, test_pane, TodoPane};

/// A quarter tile on a laptop, a half tile, and the whole window — the same
/// three every other sweep uses.
const WIDTHS: [(&str, u32, u32); 5] = [
    ("narrow", 300, 380),
    ("short", 700, 150),
    ("quarter", 470, 380),
    ("half", 700, 560),
    ("full", 1180, 760),
];

const DAY: u64 = 86_400_000;

fn item(
    id: u64,
    title: &str,
    project: Option<&str>,
    due_days: Option<i64>,
    done: bool,
) -> TodoItem {
    let now = crate::todopane::duedate::to_epoch_ms(crate::todopane::duedate::now_local()).unwrap();
    TodoItem {
        id,
        title: title.into(),
        done,
        done_ms: done.then_some(now - DAY),
        project: project.map(str::to_string),
        due_ms: due_days.map(|d| now.saturating_add_signed(d * DAY as i64)),
        due_has_time: due_days.is_some_and(|d| d == 0),
        created_ms: 1_700_000_000_000 + id,
        notified: false,
    }
}

/// A real week's list: something overdue, something due today, a long title
/// that has to wrap, an untagged scratch note, and finished work underneath.
fn week() -> Vec<TodoItem> {
    vec![
        item(1, "renew the domain", Some("admin"), Some(-2), false),
        item(2, "ship the release notes", Some("crew"), Some(0), false),
        item(
            3,
            "work out why the atlas grows on the first Retina frame and reverts the smoothing",
            Some("crew"),
            Some(1),
            false,
        ),
        item(4, "book the dentist", Some("home"), Some(6), false),
        item(5, "read the wgpu 24 changelog", None, None, false),
        item(6, "reply to the invoice thread", Some("admin"), None, false),
        item(7, "cut v0.19.73", Some("crew"), Some(-1), true),
        item(8, "clear the target dir", None, None, true),
    ]
}

/// Shoot the pane in each of its states at one size.
fn sweep_at(suffix: &str, w: u32, h: u32) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    let mut take = |name: String, p: &TodoPane| {
        if let Some(px) = shot_at(&name, w, h, 13.0, "todo", |c, r, _| {
            (p.cells(c, r), Vec::new())
        }) {
            let n = ink(&px);
            eprintln!("{name}: {n} ink px");
            out.push((name, n));
        }
    };

    let mut list = test_pane(week());
    list.sel = Some(1);
    take(format!("todo-{suffix}"), &list);

    let mut typing = test_pane(week());
    typing.insert_at_cursor("pay the hosting bill @adm");
    take(format!("todo-tag-{suffix}"), &typing);

    let mut done = test_pane(week());
    done.set_done_view(true);
    take(format!("todo-done-{suffix}"), &done);

    let empty = test_pane(Vec::new());
    take(format!("todo-empty-{suffix}"), &empty);

    out
}

/// Every todo state at every tile size. A state that draws nothing at one of
/// them is blank on somebody's screen — the assertion is only a floor; the
/// PNGs are the point.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn todo_shot_width_sweep() {
    let _g = crate::app::theme_test_guard();
    let mut any = false;
    for (suffix, w, h) in WIDTHS {
        for (name, n) in sweep_at(suffix, w, h) {
            any = true;
            assert!(n > 400, "{name} is all but blank: {n} ink pixels");
        }
    }
    if !any {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
    }
}

/// The same pane on a light page and through a green tube: the row ink, the
/// project chips and the due colours all come from the theme's roles.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn todo_shot_themes() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for (suffix, id) in [
        ("light", crew_theme::ThemeId::PaperLight),
        ("crt-green", crew_theme::ThemeId::CrtGreen),
    ] {
        crew_theme::set_theme(id);
        for (name, n) in sweep_at(suffix, 1180, 760) {
            assert!(n > 400, "{name} is all but blank: {n} ink pixels");
        }
    }
}
