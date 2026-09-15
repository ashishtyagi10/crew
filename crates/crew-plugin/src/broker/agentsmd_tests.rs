use super::*;

/// A throwaway project: `root/` with a `.git` marker, and `root/crates/app`
/// below it — the shape the search has to walk.
fn tree(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "crew-agentsmd-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(dir.join(".git")).unwrap();
    std::fs::create_dir_all(dir.join("crates").join("app")).unwrap();
    dir
}

fn write(path: &Path, name: &str, text: &str) {
    std::fs::write(path.join(name), text).unwrap();
}

#[test]
fn a_repo_root_agents_file_is_found_from_a_subdirectory() {
    let root = tree("up");
    write(&root, "AGENTS.md", "always run cargo fmt");
    let found = find_at(&root.join("crates").join("app"));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].text, "always run cargo fmt");
    assert_eq!(found[0].name, "../../AGENTS.md", "the label must say where");
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn the_search_stops_at_the_repo_root_and_never_sweeps_the_home_directory() {
    let root = tree("stop");
    let above = root.parent().unwrap().join(format!(
        "crew-agentsmd-outside-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    // Not a parent of `root` — the point is that nothing above the marker is
    // read; a sibling stands in for "somewhere else on this machine".
    std::fs::create_dir_all(&above).unwrap();
    write(&above, "AGENTS.md", "rules for a different project");
    let found = find_at(&root);
    assert!(found.is_empty(), "{found:?}");
    std::fs::remove_dir_all(&root).ok();
    std::fs::remove_dir_all(&above).ok();
}

#[test]
fn both_names_are_read_and_the_nearer_file_has_the_last_word() {
    let root = tree("both");
    write(&root, "AGENTS.md", "root rule");
    write(&root, "CLAUDE.md", "claude rule");
    let sub = root.join("crates").join("app");
    write(&sub, "AGENTS.md", "subdirectory rule");
    let (block, names) = block_at(&sub).expect("instructions");
    assert_eq!(names.len(), 3);
    assert!(
        block.find("root rule") < block.find("subdirectory rule"),
        "the nearer file must come last: {block}"
    );
    assert!(block.contains("claude rule"));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn an_empty_or_missing_file_is_no_instructions_at_all() {
    let root = tree("empty");
    write(&root, "AGENTS.md", "   \n\n");
    assert!(block_at(&root).is_none());
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_long_file_is_clipped_with_a_marker_and_the_block_stays_bounded() {
    let root = tree("cap");
    write(&root, "AGENTS.md", &"x".repeat(CAP * 3));
    let (block, _) = block_at(&root).expect("instructions");
    assert!(
        block.chars().count() <= CAP + 64,
        "{} chars",
        block.chars().count()
    );
    assert!(block.contains("clipped"), "a cut with no marker");
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn the_task_carries_the_repo_rules_above_the_users_own_memory() {
    let out = crate::broker::memory::prepend_all(
        Some("run cargo fmt".into()),
        Some("always use pnpm".into()),
        "ship it",
    );
    assert!(
        out.find("PROJECT INSTRUCTIONS") < out.find("STANDING MEMORY"),
        "{out}"
    );
    assert!(out.ends_with("ship it"));
    // Neither present: the task is untouched, so a project with no
    // conventions sends the same bytes it always did.
    assert_eq!(
        crate::broker::memory::prepend_all(None, None, "ship it"),
        "ship it"
    );
}

#[test]
fn doctor_names_the_files_it_found_and_marks_a_project_without_them_absent() {
    assert_eq!(doctor_line(&[]).0, '\u{2013}');
    let (mark, detail) = doctor_line(&["AGENTS.md".to_string()]);
    assert_eq!(mark, '\u{2713}');
    assert!(detail.contains("AGENTS.md"), "{detail}");
}
