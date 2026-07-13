use super::*;

fn tmp(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("crew-session-{name}-{}", std::process::id()))
}

#[test]
fn save_load_round_trip_keeps_existing_dirs_only() {
    let p = tmp("rt");
    let home = std::env::temp_dir().to_string_lossy().into_owned();
    save_at(
        Some(p.clone()),
        vec![home.clone(), "/definitely/not/a/dir".into()],
    );
    assert_eq!(load_at(Some(p.clone())), vec![home]);
    let _ = std::fs::remove_file(p);
}

#[test]
fn save_dedupes_and_caps_at_max_dirs() {
    let p = tmp("cap");
    let d = std::env::temp_dir().to_string_lossy().into_owned();
    // 8 copies of the same dir + itself again: dedupe leaves 1.
    save_at(Some(p.clone()), vec![d.clone(); 8]);
    assert_eq!(load_at(Some(p.clone())).len(), 1);
    // MAX_DIRS distinct dirs survive; the 7th is dropped. Created under
    // temp so the test is OS-portable (no unix-root assumptions).
    let base = tmp("cap-dirs");
    let many: Vec<String> = (0..7)
        .map(|i| {
            let d = base.join(format!("d{i}"));
            std::fs::create_dir_all(&d).unwrap();
            d.to_string_lossy().into_owned()
        })
        .collect();
    save_at(Some(p.clone()), many);
    assert_eq!(load_at(Some(p.clone())).len(), MAX_DIRS);
    let _ = std::fs::remove_file(p);
    let _ = std::fs::remove_dir_all(base);
}

#[test]
fn empty_save_removes_the_file() {
    let p = tmp("rm");
    save_at(Some(p.clone()), vec!["/".into()]);
    assert!(p.exists());
    save_at(Some(p.clone()), Vec::new());
    assert!(!p.exists());
}

#[test]
fn restore_from_opens_one_shell_per_dir_and_keeps_the_tracked_cwd() {
    let mut app = CrewApp {
        cwd: PathBuf::from("/"),
        ..Default::default()
    };
    let d1 = std::env::temp_dir().to_string_lossy().into_owned();
    app.restore_from(vec![d1.clone(), "/".into()]);
    assert_eq!(app.panes.len(), 2);
    assert!(app
        .panes
        .iter()
        .all(|p| matches!(p.content, PaneContent::Terminal(_))));
    // Spawn dirs recorded per pane; the tracked cwd is put back.
    assert_eq!(
        app.panes[0].dir.as_deref(),
        Some(std::path::Path::new(d1.as_str()))
    );
    assert_eq!(app.cwd, PathBuf::from("/"));
}

#[test]
fn restore_from_nothing_is_a_status_no_op() {
    let mut app = CrewApp::default();
    app.restore_from(Vec::new());
    assert!(app.panes.is_empty());
}

#[test]
fn unparseable_file_loads_as_empty() {
    let p = tmp("bad");
    std::fs::write(&p, "not toml [").unwrap();
    assert!(load_at(Some(p.clone())).is_empty());
    let _ = std::fs::remove_file(p);
}

#[test]
fn load_caps_and_dedupes_a_hand_edited_file() {
    // The file is user-editable: 200 entries (or duplicates) must not fork
    // 200 login shells — load applies the same cap/dedupe as save.
    let p = tmp("hostile");
    let d = std::env::temp_dir().to_string_lossy().into_owned();
    let dirs: Vec<String> = std::iter::repeat_n(d, 200).collect();
    let text = toml::to_string(&Session { dirs }).unwrap();
    std::fs::write(&p, text).unwrap();
    assert_eq!(load_at(Some(p.clone())).len(), 1, "dupes collapse");
    let base = tmp("hostile-dirs");
    let dirs: Vec<String> = (0..200)
        .map(|i| {
            let d = base.join(format!("d{i}"));
            std::fs::create_dir_all(&d).unwrap();
            d.to_string_lossy().into_owned()
        })
        .collect();
    std::fs::write(&p, toml::to_string(&Session { dirs }).unwrap()).unwrap();
    assert_eq!(load_at(Some(p.clone())).len(), MAX_DIRS);
    let _ = std::fs::remove_file(p);
    let _ = std::fs::remove_dir_all(base);
}
