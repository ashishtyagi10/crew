//! How the tree draws.
use super::*;
use crate::explorer::Explorer;

fn rows_of(cells: &[CellView]) -> Vec<String> {
    let max = cells.iter().map(|c| c.row).max().unwrap_or(0);
    (0..=max)
        .map(|r| {
            let mut v: Vec<&CellView> = cells.iter().filter(|c| c.row == r).collect();
            v.sort_by_key(|c| c.col);
            let mut s = String::new();
            for c in v {
                while s.chars().count() < usize::from(c.col) {
                    s.push(' ');
                }
                s.push(c.c);
            }
            s.trim_end().to_string()
        })
        .collect()
}

#[test]
fn a_tree_draws_with_signs_guides_and_a_capitalised_root() {
    let _g = crate::app::theme_test_guard();
    let root = crate::explorer::tests::tree("draw", &["src/a.rs", "src/deep/b.rs", "x.md"]);
    let mut e = Explorer::default();
    let items = e.items(&root);
    e.toggle(&items[1]);
    let items = e.items(&root);
    e.toggle(&items[2]); // src/deep
    let items = e.items(&root);
    let drawn = rows_of(&cells(&items, 0, Marks::default(), (30, 20)));
    assert!(drawn[0].contains("EXPLORER"), "{drawn:?}");
    assert!(
        drawn[1].starts_with(" \u{2212} CREW-EXPLORER-DRAW"),
        "{drawn:?}"
    );
    assert_eq!(drawn[2], "   \u{2212} src");
    assert_eq!(drawn[3], "   \u{2502} \u{2212} deep");
    assert_eq!(drawn[4], "   \u{2502} \u{2502}   b.rs");
    assert_eq!(drawn[5], "   \u{2502}   a.rs");
    assert_eq!(drawn[6], "     x.md");
}

/// Scrolled, item `k` is on row `k - scroll + TOP`, and nothing is drawn past the panel.
#[test]
fn a_scrolled_tree_starts_at_its_scroll_and_stops_at_the_panel() {
    let _g = crate::app::theme_test_guard();
    let names: Vec<String> = (0..30).map(|i| format!("f{i:02}")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let root = crate::explorer::tests::tree("scroll", &refs);
    let items = Explorer::default().items(&root);
    let c = cells(&items, 5, Marks::default(), (20, 10));
    let drawn = rows_of(&c);
    assert_eq!(drawn[1].trim(), items[5].name);
    assert!(c.iter().all(|c| c.row < 10 && c.col < 20));
}

/// Off-screen: crew's own tree mid-find — `ca` on `Cargo.toml`, then `caz` matching nothing.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn explorer_find_shot() {
    use crew_theme::ThemeId;
    let _g = crate::app::theme_test_guard();
    for (id, typed) in [(ThemeId::TokyoNight, "ca"), (ThemeId::Porcelain, "caz")] {
        let name = format!("explorer-find-{id:?}-{typed}").to_lowercase();
        let prep = move |a: &mut crate::app::CrewApp| {
            crate::explorerfind::TEST_NOW.with(|t| t.set(Some(0)));
            a.explorer_keys_toggle();
            for c in typed.chars() {
                a.explorer_tree_key(&winit::keyboard::Key::Character(c.to_string().into()), true);
            }
        };
        let opacity = crate::tubesheer::sheer(id.theme());
        crate::crtglassshot_tests::window_with(&name, id, opacity, &[], &prep);
    }
}

/// A wide prefix is clipped by display width, so the legend never outgrows the rule.
#[test]
fn a_wide_prefix_is_clipped_by_width() {
    let room = crate::boxdraw::title_budget(30).saturating_sub("EXPLORER find: ".len());
    let l = crate::explorerfind::legend(&"日".repeat(20), 30);
    assert!(l.starts_with("find: \u{2026}"), "{l}");
    assert!(crate::chatwidth::str_w(&l) - "find: ".len() <= room, "{l}");
}

/// Two roots of one name — a pinned `conf` beside another — each say where they are, muted;
/// a root whose name is its own says nothing more. A closed folder wears `+`.
#[test]
fn same_named_roots_show_their_parent_folders() {
    let _g = crate::app::theme_test_guard();
    let a = crate::explorer::tests::tree("twina", &["conf/x/k.properties"]);
    let b = crate::explorer::tests::tree("twinb", &["conf/y.properties"]);
    let solo = crate::explorer::tests::tree("twinsolo", &["z.txt"]);
    let e = Explorer::default();
    let items: Vec<_> = [a.join("conf"), b.join("conf"), solo.clone()]
        .iter()
        .flat_map(|r| e.items(r))
        .collect();
    let cells = cells(&items, 0, Marks::default(), (200, 20));
    let drawn = rows_of(&cells);
    let short = |p: &std::path::Path| crate::disktile::short_path(p);
    assert_eq!(
        drawn[1],
        format!(" \u{2212} CONF  {}", short(&a)),
        "{drawn:?}"
    );
    assert_eq!(drawn[2], "   + x", "a closed folder: {drawn:?}");
    assert_eq!(
        drawn[3],
        format!(" \u{2212} CONF  {}", short(&b)),
        "{drawn:?}"
    );
    let solo_row = drawn
        .iter()
        .find(|r| r.contains("CREW-EXPLORER-TWINSOLO"))
        .unwrap();
    assert!(
        !solo_row.contains(&short(solo.parent().unwrap())),
        "{solo_row}"
    );
    let muted = crew_theme::theme().text_muted;
    let first = short(&a).chars().next().unwrap();
    let at = cells.iter().find(|c| c.row == 1 && c.col == 9).unwrap();
    assert_eq!((at.c, at.fg), (first, muted), "the parent is muted");
}

#[test]
fn a_parent_too_long_keeps_its_end() {
    assert_eq!(tail("~/work/app", 20), "~/work/app");
    assert_eq!(tail("/var/folders/x/T/crew-a", 10), "\u{2026}/T/crew-a");
    assert_eq!(crate::chatwidth::str_w(&tail("/a/very/long/path", 6)), 6);
}
