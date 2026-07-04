use super::*;
use crate::broker::skills::parse;

fn skill_with(body: &str, dir: Option<&str>) -> crate::broker::skills::Skill {
    let mut s = parse(body, "demo", "user");
    s.path = std::path::PathBuf::from("/skills/demo/SKILL.md");
    s.dir = dir.map(std::path::PathBuf::from);
    s
}

#[test]
fn small_flat_skill_frames_exactly_as_before() {
    let s = skill_with("Check unsafe blocks.", None);
    assert_eq!(
        framed(&s, "review foo.rs", true),
        "SKILL \u{201c}demo\u{201d} \u{2014} follow this playbook:\n\
         Check unsafe blocks.\n\nTASK:\nreview foo.rs"
    );
}

#[test]
fn directory_skill_frame_points_at_its_supporting_files() {
    let s = skill_with("Check unsafe blocks.", Some("/skills/demo"));
    let f = framed(&s, "review foo.rs", true);
    assert!(f.contains("Supporting files: /skills/demo"), "got: {f}");
    assert!(f.contains("@tool sys:read_file"), "got: {f}");
    assert!(f.contains("@tool sys:run"), "got: {f}");
    assert!(f.ends_with("TASK:\nreview foo.rs"), "got: {f}");
}
