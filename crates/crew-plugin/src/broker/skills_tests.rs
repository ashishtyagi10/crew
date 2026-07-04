use super::*;

fn tmpdir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!(
        "crew-skills-{tag}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn parse_reads_frontmatter_name_and_description() {
    let s = parse(
        "---\nname: Review Checklist\ndescription: strict Rust review\n---\nCheck unsafe blocks.",
        "file-stem",
        "user",
    );
    assert_eq!(s.name, "review-checklist");
    assert_eq!(s.description, "strict Rust review");
    assert_eq!(s.body, "Check unsafe blocks.");
}

#[test]
fn parse_falls_back_to_stem_and_first_line() {
    let s = parse(
        "Always write tests first.\nMore detail.",
        "TDD Loop",
        "project",
    );
    assert_eq!(s.name, "tdd-loop");
    assert_eq!(s.description, "Always write tests first.");
    assert_eq!(s.body, "Always write tests first.\nMore detail.");
}

#[test]
fn parse_tolerates_unclosed_frontmatter() {
    let s = parse("---\nname: broken", "stem", "user");
    assert_eq!(s.name, "stem");
    assert!(s.body.contains("name: broken"));
}

#[test]
fn load_dir_reads_only_md_files_sorted() {
    let d = tmpdir("loaddir");
    std::fs::write(d.join("b.md"), "beta").unwrap();
    std::fs::write(d.join("a.md"), "alpha").unwrap();
    std::fs::write(d.join("ignored.txt"), "nope").unwrap();
    let skills = load_dir(&d, "user");
    let names: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["a", "b"]);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn load_dir_of_missing_path_is_empty() {
    assert!(load_dir(std::path::Path::new("/nonexistent/xyz"), "user").is_empty());
}

#[test]
fn load_dir_sees_files_added_after_a_previous_scan() {
    // Hot-reload contract: skills re-read the directory on every use, so a
    // file dropped in while crew runs shows up without a restart.
    let d = tmpdir("fresh");
    std::fs::write(d.join("a.md"), "alpha").unwrap();
    assert_eq!(load_dir(&d, "user").len(), 1);
    std::fs::write(d.join("b.md"), "beta").unwrap();
    let names: Vec<String> = load_dir(&d, "user").into_iter().map(|s| s.name).collect();
    assert_eq!(names, vec!["a", "b"]);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn merge_lets_project_override_user() {
    let user = vec![
        parse("user body", "shared", "user"),
        parse("only user", "solo", "user"),
    ];
    let project = vec![parse("project body", "shared", "project")];
    let all = merge(user, project);
    let shared = all.iter().find(|s| s.name == "shared").unwrap();
    assert_eq!(
        (shared.origin, shared.body.as_str()),
        ("project", "project body")
    );
    assert_eq!(all.len(), 2);
}

#[test]
fn list_report_explains_when_empty_and_lists_when_not() {
    assert!(list_report(&[]).contains("~/.config/crew/skills/"));
    let r = list_report(&[parse(
        "---\nname: x\ndescription: d\n---\nbody",
        "x",
        "user",
    )]);
    assert!(r.contains("x \u{2014} d (user)"), "got: {r}");
}

#[test]
fn framed_puts_playbook_before_task() {
    let s = parse("playbook text", "guide", "user");
    let f = framed(&s, "do the thing");
    let (pb, task) = (
        f.find("playbook text").unwrap(),
        f.find("do the thing").unwrap(),
    );
    assert!(pb < task);
}

#[test]
fn skill_cmd_without_args_prints_usage() {
    let mut session = Session::new();
    let mut got = Vec::new();
    skill_cmd(&mut session, "", &mut |ev| {
        got.push(ev);
        Ok(())
    })
    .unwrap();
    match &got[0] {
        PluginEvent::Message { text, .. } => assert!(text.contains("usage: /skill")),
        other => panic!("unexpected event: {other:?}"),
    }
}

#[test]
fn skill_cmd_reports_an_unknown_skill() {
    let mut session = Session::new();
    let mut got = Vec::new();
    skill_cmd(&mut session, "no-such-skill do it", &mut |ev| {
        got.push(ev);
        Ok(())
    })
    .unwrap();
    match &got[0] {
        PluginEvent::Message { text, .. } => {
            assert!(text.contains("unknown skill"), "got: {text}")
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[test]
fn load_dir_reads_a_directory_skill() {
    let d = tmpdir("dirskill");
    std::fs::create_dir_all(d.join("My Skill")).unwrap();
    std::fs::write(d.join("My Skill").join("SKILL.md"), "Do the thing.").unwrap();
    let skills = load_dir(&d, "user");
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "my-skill");
    assert_eq!(skills[0].body, "Do the thing.");
    assert_eq!(skills[0].path, d.join("My Skill").join("SKILL.md"));
    assert_eq!(skills[0].dir.as_deref(), Some(d.join("My Skill").as_path()));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn directory_skill_frontmatter_name_wins_over_dir_name() {
    let d = tmpdir("dirname");
    std::fs::create_dir_all(d.join("foo")).unwrap();
    std::fs::write(
        d.join("foo").join("SKILL.md"),
        "---\nname: Real Name\n---\nbody",
    )
    .unwrap();
    let skills = load_dir(&d, "project");
    assert_eq!(skills[0].name, "real-name");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn subdir_without_skill_md_is_skipped_and_flat_files_get_path() {
    let d = tmpdir("mixed");
    std::fs::create_dir_all(d.join("not-a-skill")).unwrap();
    std::fs::write(d.join("flat.md"), "flat body").unwrap();
    let skills = load_dir(&d, "user");
    let names: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["flat"]);
    assert_eq!(skills[0].path, d.join("flat.md"));
    assert_eq!(skills[0].dir, None);
    let _ = std::fs::remove_dir_all(&d);
}
