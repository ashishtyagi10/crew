//! F6's box: Enter as offered moves into the other panel, a new name renames
//! in place, a path moves there, and nothing is ever overwritten.
use super::*;
use crate::farpane::keys::FarAction;

/// A pane on a fresh temp dir holding `f.txt` and `sub/`, both panels on it
/// (how `/far` opens), `f.txt` selected on the left.
fn fixture(key: &str) -> (PathBuf, FarPane) {
    let base = std::env::temp_dir().join(format!("crew_far_move_{key}"));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("sub")).unwrap();
    std::fs::write(base.join("f.txt"), b"x").unwrap();
    let mut p = FarPane::new(base.clone());
    p.left.sel = p
        .left
        .entries
        .iter()
        .position(|e| e.name == "f.txt")
        .unwrap();
    (base, p)
}

/// Open the box, type `input` over what it offered, press Enter.
fn f6(p: &mut FarPane, input: Option<&str>) -> Option<FarAction> {
    assert!(open_move(p).is_none(), "F6 opens the box, it does not act");
    if let Some(text) = input {
        p.prompt.as_mut().unwrap().input = text.into();
    }
    submit_prompt(p)
}

fn status(a: Option<FarAction>) -> String {
    match a {
        Some(FarAction::Status(s)) => s,
        _ => panic!("expected a status"),
    }
}

#[test]
fn f6_opens_a_box_offering_the_other_panel_s_folder_and_moves_nothing() {
    let (base, mut p) = fixture("open");
    p.right.loc = Location::local(&base.join("sub"));
    assert!(open_move(&mut p).is_none());
    let pr = p.prompt.as_ref().expect("the box is open");
    assert!(pr.floats());
    assert!(pr.input.ends_with("sub/"), "{:?}", pr.input);
    assert!(matches!(&pr.kind, PromptKind::Move { name, .. } if name == "f.txt"));
    assert!(base.join("f.txt").exists(), "nothing moves until Enter");
}

#[test]
fn enter_as_offered_moves_into_the_other_panel() {
    let (base, mut p) = fixture("offered");
    p.right.loc = Location::local(&base.join("sub"));
    p.right.reload();
    let said = status(f6(&mut p, None));
    assert!(base.join("sub/f.txt").exists(), "{said}");
    assert!(!base.join("f.txt").exists());
    assert!(p.prompt.is_none(), "the box closes");
    assert!(
        p.right.entries.iter().any(|e| e.name == "f.txt"),
        "both panels reload"
    );
}

#[test]
fn a_new_name_renames_in_place() {
    let (base, mut p) = fixture("rename");
    let said = status(f6(&mut p, Some("g.txt")));
    assert!(base.join("g.txt").exists(), "{said}");
    assert!(!base.join("f.txt").exists());
    assert!(said.contains("moved \u{2018}f.txt\u{2019}"), "{said}");
}

/// Both panels on one folder, Enter as offered: nothing to move to — the
/// box says how to rename rather than failing on "already exists".
#[test]
fn same_folder_as_offered_says_to_type_a_name() {
    let (base, mut p) = fixture("same");
    let said = status(f6(&mut p, None));
    assert!(said.contains("type a new name"), "{said}");
    assert!(base.join("f.txt").exists());
}

#[test]
fn a_path_moves_there_into_a_folder_or_as_a_new_name() {
    let (base, mut p) = fixture("path");
    status(f6(&mut p, Some("sub/")));
    assert!(
        base.join("sub/f.txt").exists(),
        "relative to the active panel"
    );
    let (base, mut p) = fixture("path_named");
    status(f6(&mut p, Some("sub/h.txt")));
    assert!(
        base.join("sub/h.txt").exists(),
        "a path naming no folder is the new name"
    );
    let (base, mut p) = fixture("path_abs");
    let abs = base.join("sub").to_string_lossy().into_owned();
    status(f6(&mut p, Some(&abs)));
    assert!(
        base.join("sub/f.txt").exists(),
        "an existing folder is moved into"
    );
}

#[test]
fn an_existing_target_is_never_overwritten() {
    let (base, mut p) = fixture("clobber");
    std::fs::write(base.join("g.txt"), b"keep").unwrap();
    let said = status(f6(&mut p, Some("g.txt")));
    assert!(said.contains("already exists"), "{said}");
    assert_eq!(std::fs::read(base.join("g.txt")).unwrap(), b"keep");
    assert!(base.join("f.txt").exists());
}

#[test]
fn an_emptied_input_does_nothing_and_the_parent_row_is_refused() {
    let (base, mut p) = fixture("empty");
    assert!(f6(&mut p, Some("   ")).is_none());
    assert!(base.join("f.txt").exists());
    p.left.sel = p.left.entries.iter().position(|e| e.is_parent).unwrap_or(0);
    if p.left.entries[p.left.sel].is_parent {
        assert!(status(open_move(&mut p)).contains("‘..’"));
        assert!(p.prompt.is_none());
    }
}

#[test]
fn the_offered_folder_reads_as_a_folder() {
    let loc = Location::local(std::path::Path::new("/tmp/x"));
    assert_eq!(folder_text(&loc), "/tmp/x/");
    assert_eq!(
        folder_text(&Location::local(std::path::Path::new("/"))),
        "/"
    );
}

/// Drawn: a box over the panels says what moves, shows the offered folder
/// with the pane's only caret, and says what Enter does — at a full tile and
/// at a narrow one, where the hint shortens rather than spilling.
#[test]
fn the_box_is_drawn_over_the_panels_with_the_only_caret() {
    let _g = crate::app::theme_test_guard();
    let (_base, mut p) = fixture("draw");
    open_move(&mut p);
    for (cols, hint) in [
        (100u16, "a new name renames \u{b7} Esc"),
        (40, "Enter moves"),
    ] {
        let cells = crate::farpane::render::render_in(&p, cols, 24, true);
        let rows: Vec<String> = (0..24u16)
            .map(|r| {
                let mut row: Vec<_> = cells.iter().filter(|c| c.row == r).collect();
                row.sort_by_key(|c| c.col);
                row.iter().map(|c| c.c).collect()
            })
            .collect();
        // Blank cells are not emitted, so compare without spaces.
        let has = |s: &str| rows.iter().any(|r| r.contains(&s.replace(' ', "")));
        assert!(has("Rename or move"), "{cols}: {rows:#?}");
        assert!(has("\u{2018}f.txt\u{2019} to:"), "{cols}: {rows:#?}");
        assert!(has(hint), "{cols}: {rows:#?}");
        // (A narrow key row drops labels to fit, so only the wide one.)
        assert!(
            cols < 100 || has("RenMov"),
            "the key row stays under the box"
        );
        let carets = cells.iter().filter(|c| c.c == '\u{258f}').count();
        assert_eq!(carets, 1, "{cols}: the box's input owns the caret");
    }
}
