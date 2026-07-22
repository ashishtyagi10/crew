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

#[test]
fn min_flag_round_trips_through_toml_and_false_is_omitted() {
    // Pins the serde attrs (default + skip_serializing_if Not::not): a
    // future attr edit must not silently break restore-minimized.
    let p = tmp("min");
    let mut hidden = SavedPane::shell(tmp_dir_str());
    hidden.min = true;
    save_at(Some(p.clone()), vec![hidden.clone(), SavedPane::crew()]);
    let text = std::fs::read_to_string(&p).unwrap();
    assert_eq!(text.matches("min = true").count(), 1, "{text}");
    assert!(!text.contains("min = false"), "false is skipped: {text}");
    assert_eq!(
        load_at(Some(p.clone())),
        vec![hidden, SavedPane::crew()],
        "min survives the round trip; absent min loads as false"
    );
    let _ = std::fs::remove_file(p);
}

#[test]
fn remote_far_pane_round_trips_and_false_is_omitted() {
    // Task 12: a remote Far pane's `dir` is an rclone address, and `remote`
    // must round-trip alongside it — false (local, the pre-Task-12 shape)
    // stays omitted from the file so old readers still see a bare `dir`.
    let p = tmp("remote");
    save_at(
        Some(p.clone()),
        vec![
            SavedPane::far_remote("gdrive:Photos".into()),
            SavedPane::far(tmp_dir_str()),
        ],
    );
    let text = std::fs::read_to_string(&p).unwrap();
    assert_eq!(text.matches("remote = true").count(), 1, "{text}");
    assert!(!text.contains("remote = false"), "false is skipped: {text}");
    assert_eq!(
        load_at(Some(p.clone())),
        vec![
            SavedPane::far_remote("gdrive:Photos".into()),
            SavedPane::far(tmp_dir_str())
        ]
    );
    let _ = std::fs::remove_file(p);
}

#[test]
fn same_dir_shells_with_different_min_both_survive_dedupe() {
    // Two real panes (one minimized) in the same cwd must not collapse into
    // one arbitrary survivor — min is part of the dedupe key.
    let p = tmp("minkey");
    let mut hidden = SavedPane::shell(tmp_dir_str());
    hidden.min = true;
    save_at(
        Some(p.clone()),
        vec![hidden, SavedPane::shell(tmp_dir_str())],
    );
    assert_eq!(load_at(Some(p.clone())).len(), 2);
    let _ = std::fs::remove_file(p);
}
