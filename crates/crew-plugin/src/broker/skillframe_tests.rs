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

fn big_body() -> String {
    // > 8 KB, with headings and an intro paragraph.
    let mut b = String::from("This skill explains everything.\n\n## Setup\n");
    b.push_str(&"filler line for bulk\n".repeat(500));
    b.push_str("### Details\nmore\n## Usage\nfinal\n");
    b
}

#[test]
fn oversized_body_frames_as_outline_and_path() {
    let s = skill_with(&big_body(), None);
    let f = framed(&s, "do it", true);
    assert!(
        f.len() < 2048,
        "pointer frame stays small, got {} bytes",
        f.len()
    );
    assert!(f.contains("SKILL \u{201c}demo\u{201d} \u{2014} This skill explains everything."));
    assert!(
        f.contains("Outline:\n## Setup\n### Details\n## Usage"),
        "got: {f}"
    );
    assert!(
        f.contains("Full playbook: /skills/demo/SKILL.md"),
        "got: {f}"
    );
    assert!(f.contains("@tool sys:read_file"), "got: {f}");
    assert!(f.ends_with("TASK:\ndo it"), "got: {f}");
}

#[test]
fn oversized_body_with_no_headings_frames_as_intro_and_path() {
    let body = "line\n".repeat(2000);
    let s = skill_with(&body, None);
    let f = framed(&s, "do it", true);
    assert!(!f.contains("Outline:"), "got: {f}");
    assert!(f.contains("Full playbook:"), "got: {f}");
    // Intro is byte-clipped ~1 KB, so the frame stays small.
    assert!(f.len() < 2048, "got {} bytes", f.len());
}

#[test]
fn sys_tools_off_falls_back_to_full_inline() {
    let s = skill_with(&big_body(), None);
    let f = framed(&s, "do it", false);
    assert!(f.contains("follow this playbook"), "got: {f}");
    assert!(f.contains("filler line for bulk"), "got: {f}");
    assert!(!f.contains("Full playbook:"), "got: {f}");
}

#[test]
fn small_body_still_inlines_even_with_sys_on() {
    let s = skill_with("tiny", None);
    assert!(framed(&s, "t", true).contains("follow this playbook"));
}
