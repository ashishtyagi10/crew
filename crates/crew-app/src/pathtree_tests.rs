use super::*;

fn list(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// The file name's start beats a match inside the path, which beats letters
/// in order; ties go to the shorter path.
#[test]
fn name_prefix_then_substring_then_subsequence() {
    let f = list(&[
        "docs/hit-list.md",
        "src/explorerhit.rs",
        "src/hit.rs",
        "src/deep/nested/hit.rs",
        "src/ex_plorer_h_i_t.rs",
    ]);
    let got: Vec<String> = best(&f, "hit").into_iter().map(|(p, _)| p).collect();
    assert_eq!(
        got,
        [
            "src/hit.rs",
            "docs/hit-list.md",
            "src/deep/nested/hit.rs",
            "src/explorerhit.rs",
            "src/ex_plorer_h_i_t.rs",
        ]
    );
    let got: Vec<String> = best(&f, "exhit").into_iter().map(|(p, _)| p).collect();
    assert_eq!(got, ["src/explorerhit.rs", "src/ex_plorer_h_i_t.rs"]);
}

/// The marks are where the query sits, so a fuzzy row explains itself.
#[test]
fn marks_name_prefix_substring_and_subsequence() {
    let marks = |p: &str, q: &str| score(p, q).expect("a match").1;
    assert_eq!(marks("src/hit.rs", "hit"), vec![4, 5, 6]);
    assert_eq!(marks("src/explorerhit.rs", "rerh"), vec![9, 10, 11, 12]);
    assert_eq!(marks("src/explorerhit.rs", "exh"), vec![4, 5, 12]);
    assert_eq!(marks("A/B.rs", "b"), vec![2], "case folded");
    assert!(score("src/hit.rs", "xyz").is_none());
}

/// Letters in order inside the file name beat letters smeared across the
/// path — the shot of `/view exhit` over crew's tree listed
/// `inbox/html/corpus/github.md` second before this tier existed.
#[test]
fn letters_in_the_name_beat_letters_across_the_path() {
    let f = list(&[
        "crates/crew-app/src/inbox/html/corpus/github.md",
        "zz/explorerhit.rs",
    ]);
    let got: Vec<String> = best(&f, "exhit").into_iter().map(|(p, _)| p).collect();
    assert_eq!(
        got,
        ["zz/explorerhit.rs"],
        "the smeared path is noise under a real hit"
    );
    let alone = list(&["crates/crew-app/src/inbox/html/corpus/github.md"]);
    assert_eq!(
        best(&alone, "exhit").len(),
        1,
        "a guess when it is all there is"
    );
    // Inside a tier the tighter match leads, whatever the length.
    let f = list(&["e_x_h_i_t.rs", "a/much/longer/path/exhibit.rs"]);
    let got: Vec<String> = best(&f, "exht").into_iter().map(|(p, _)| p).collect();
    assert_eq!(got[0], "a/much/longer/path/exhibit.rs");
}

#[test]
fn rows_are_capped() {
    let many: Vec<String> = (0..100).map(|i| format!("f{i}.rs")).collect();
    assert_eq!(best(&many, "f").len(), MAX_ROWS);
}

/// The first read starts a walk and answers nothing; once it lands the same
/// root answers from it without walking again, and a new root starts over.
#[test]
fn walks_off_thread_once_per_root() {
    let base = std::env::temp_dir().join(format!("crew_pathtree_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("a/b")).unwrap();
    std::fs::write(base.join("a/b/deep.rs"), b"x").unwrap();
    assert!(files(&base, 1).is_none(), "first read only starts the walk");
    let mut got = None;
    for _ in 0..200 {
        got = files(&base, 2);
        if got.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(got.expect("walk landed").as_slice(), ["a/b/deep.rs"]);
    // Inside FRESH_MS: no new walk, the same list.
    std::fs::write(base.join("new.rs"), b"x").unwrap();
    assert_eq!(files(&base, 3).unwrap().len(), 1);
    let other = base.join("a");
    assert!(files(&other, 4).is_none(), "another root starts over");
    let _ = std::fs::remove_dir_all(&base);
}

/// The picker over crew's own tree, typed `/view exhit` — folder rows (none
/// match) then tree rows, the matched letters marked.
/// `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew go_to_file_shot -- --ignored --nocapture`
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn go_to_file_shot() {
    let _g = crate::app::theme_test_guard();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let t0 = std::time::Instant::now();
    let mut items = Vec::new();
    for _ in 0..400 {
        items = crate::pathmenu::rows("/view exhit", &root).expect("a path command");
        if !items.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let walked = t0.elapsed();
    let files = files(&root, 0).expect("walked");
    let t1 = std::time::Instant::now();
    let _ = best(&files, "exhit");
    eprintln!(
        "walk {walked:?} for {} files; rank {:?}",
        files.len(),
        t1.elapsed()
    );
    for (name, w) in [("goto-narrow", 380), ("goto-half", 640)] {
        let Some(rows) = crate::pickshot_tests::menu_shot(name, "files", &items, 0, w) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(
            rows.iter().any(|r| r.contains("explorerhit.rs")),
            "{rows:?}"
        );
    }
}
