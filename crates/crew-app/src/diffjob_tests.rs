use super::*;

/// One repo, one file: re-running `/diff` overwrites its own review instead
/// of leaving a new one in the temp directory every time.
#[test]
fn the_same_repo_always_writes_the_same_file() {
    let a = temp_path(Path::new("/Users/me/code/crew"));
    assert_eq!(a, temp_path(Path::new("/Users/me/code/crew")));
    assert_ne!(a, temp_path(Path::new("/Users/me/code/other")));
}

/// The viewer picks its rung by extension, so the name has to end in `.diff`
/// or the review renders as plain text.
#[test]
fn the_review_is_named_as_a_diff() {
    let p = temp_path(Path::new("/tmp/x"));
    assert_eq!(p.extension().and_then(|e| e.to_str()), Some("diff"));
    assert!(p.starts_with(std::env::temp_dir()));
}

/// A path with separators, spaces and dots becomes one filename component —
/// a name that is still a path is a write into a directory that may not exist.
#[test]
fn a_directory_full_of_separators_becomes_one_name() {
    let p = temp_path(Path::new("/Users/me/my code/../crew.git"));
    let name = p.file_name().unwrap().to_string_lossy().into_owned();
    assert!(!name.contains('/'), "{name}");
    assert!(!name.contains(' '), "{name}");
    assert_eq!(p.parent(), Some(std::env::temp_dir().as_path()));
}

/// Two long paths differing only near their end still get different names.
#[test]
fn long_paths_differ_by_their_tail() {
    let base = "/Users/someone/very/deeply/nested/workspace/directory/tree/that/goes/on";
    assert_ne!(
        temp_path(Path::new(&format!("{base}/alpha"))),
        temp_path(Path::new(&format!("{base}/beta")))
    );
}

#[test]
fn a_fresh_job_is_idle_and_has_nothing_to_take() {
    let mut j = DiffJob::default();
    assert!(!j.busy());
    assert!(j.take().is_none());
}

/// A real read of a real repo: crew's own. The review is written, named as a
/// diff, and the job goes idle again — and taking twice yields nothing the
/// second time, so one read can never open two panes.
#[test]
fn a_read_of_a_repo_lands_in_a_file_exactly_once() {
    let mut j = DiffJob::default();
    j.start(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    assert!(j.busy());
    let mut done = None;
    for _ in 0..200 {
        if let Some(d) = j.take() {
            done = Some(d);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let done = done.expect("the read finished within five seconds");
    assert!(!j.busy(), "the job stayed busy after finishing");
    assert!(j.take().is_none(), "the same read was taken twice");
    // A clean tree is a legitimate outcome and says so rather than opening an
    // empty pane; a dirty one wrote a file.
    match done {
        Ok(p) => assert!(p.is_file()),
        Err(e) => assert!(e.contains("clean"), "{e}"),
    }
}
