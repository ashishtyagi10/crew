use super::*;

fn tmp(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("crew-session-{name}-{}", std::process::id()))
}

fn tmp_dir_str() -> String {
    std::env::temp_dir().to_string_lossy().into_owned()
}

#[test]
fn save_load_round_trip_keeps_restorable_panes_only() {
    let p = tmp("rt");
    save_at(
        Some(p.clone()),
        vec![
            SavedPane::shell(tmp_dir_str()),
            SavedPane::shell("/definitely/not/a/dir".into()),
            SavedPane::far(tmp_dir_str()),
            SavedPane::crew(),
        ],
    );
    let got = load_at(Some(p.clone()));
    assert_eq!(
        got,
        vec![
            SavedPane::shell(tmp_dir_str()),
            SavedPane::far(tmp_dir_str()),
            SavedPane::crew(),
        ]
    );
    let _ = std::fs::remove_file(p);
}

#[test]
fn v1_dirs_files_still_load_as_shells() {
    // v0.5.73–74 wrote `dirs = [...]` — must keep restoring after upgrade.
    let p = tmp("v1");
    std::fs::write(&p, format!("dirs = [{:?}]\n", tmp_dir_str())).unwrap();
    assert_eq!(
        load_at(Some(p.clone())),
        vec![SavedPane::shell(tmp_dir_str())]
    );
    let _ = std::fs::remove_file(p);
}

#[test]
fn unknown_kinds_are_skipped_not_fatal() {
    // A newer build's kind must not fail the whole load on an older one.
    let p = tmp("fwd");
    std::fs::write(
        &p,
        format!(
            "[[panes]]\nkind = \"hologram\"\n\n[[panes]]\nkind = \"shell\"\ndir = {:?}\n",
            tmp_dir_str()
        ),
    )
    .unwrap();
    assert_eq!(
        load_at(Some(p.clone())),
        vec![SavedPane::shell(tmp_dir_str())]
    );
    let _ = std::fs::remove_file(p);
}

#[test]
fn same_dir_different_kind_both_survive_dedupe() {
    let p = tmp("kinds");
    save_at(
        Some(p.clone()),
        vec![
            SavedPane::shell(tmp_dir_str()),
            SavedPane::far(tmp_dir_str()),
            SavedPane::shell(tmp_dir_str()),
        ],
    );
    assert_eq!(load_at(Some(p.clone())).len(), 2, "kind is part of the key");
    let _ = std::fs::remove_file(p);
}

#[test]
fn save_and_load_both_cap_at_max_panes() {
    let p = tmp("cap");
    let base = tmp("cap-dirs");
    let many: Vec<SavedPane> = (0..8)
        .map(|i| {
            let d = base.join(format!("d{i}"));
            std::fs::create_dir_all(&d).unwrap();
            SavedPane::shell(d.to_string_lossy().into_owned())
        })
        .collect();
    save_at(Some(p.clone()), many.clone());
    assert_eq!(load_at(Some(p.clone())).len(), MAX_PANES);
    // A hand-edited over-cap file is bounded on load too.
    let session = many
        .iter()
        .map(|sp| {
            format!(
                "[[panes]]\nkind = \"shell\"\ndir = {:?}\n",
                sp.dir.clone().unwrap()
            )
        })
        .collect::<String>();
    std::fs::write(&p, session).unwrap();
    assert_eq!(load_at(Some(p.clone())).len(), MAX_PANES);
    let _ = std::fs::remove_file(p);
    let _ = std::fs::remove_dir_all(base);
}

#[test]
fn empty_save_removes_the_file() {
    let p = tmp("rm");
    save_at(Some(p.clone()), vec![SavedPane::crew()]);
    assert!(p.exists());
    save_at(Some(p.clone()), Vec::new());
    assert!(!p.exists());
}

#[test]
fn unparseable_file_loads_as_empty() {
    let p = tmp("bad");
    std::fs::write(&p, "not toml [").unwrap();
    assert!(load_at(Some(p.clone())).is_empty());
    let _ = std::fs::remove_file(p);
}
