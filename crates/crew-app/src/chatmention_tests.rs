use super::*;

fn entries(paths: &[&str]) -> Vec<MentionEntry> {
    paths
        .iter()
        .map(|p| MentionEntry::File(p.to_string()))
        .collect()
}

#[test]
fn pending_mention_is_the_trailing_at_token() {
    assert_eq!(pending_mention("hey @sr"), Some("sr"));
    assert_eq!(pending_mention("hey @"), Some(""));
    assert_eq!(pending_mention("@coder fix @src/ma"), Some("src/ma"));
}

#[test]
fn leading_token_is_the_agent_selector_not_a_mention() {
    assert_eq!(pending_mention("@coder"), None);
    assert_eq!(pending_mention("@pl"), None);
}

#[test]
fn plain_text_and_ended_tokens_are_no_mention() {
    assert_eq!(pending_mention("hello"), None);
    assert_eq!(pending_mention("hey @src/main.rs "), None);
    assert_eq!(pending_mention("mail a@b"), None); // '@' mid-word is not a mention
    assert_eq!(pending_mention(""), None);
}

#[test]
fn filter_ranks_name_prefix_over_substring_over_subsequence() {
    let e = entries(&["docs/main-notes.md", "src/main.rs", "crates/app/mod.rs"]);
    let got = filter(&e, "main");
    assert_eq!(got[0].label(), "src/main.rs"); // filename prefix
    assert_eq!(got[1].label(), "docs/main-notes.md"); // path substring
    let got = filter(&e, "camod");
    assert_eq!(
        got.iter().map(|m| m.label()).collect::<Vec<_>>(),
        vec!["crates/app/mod.rs"]
    ); // subsequence
}

#[test]
fn filter_empty_query_lists_everything_and_misses_are_dropped() {
    let e = entries(&["a.rs", "b.rs"]);
    assert_eq!(filter(&e, "").len(), 2);
    assert!(filter(&e, "zzz").is_empty());
}

#[test]
fn filter_sections_agents_then_skills_then_files() {
    let mut e = entries(&["review-checklist.md"]);
    e.push(MentionEntry::Agent {
        name: "reviewer".into(),
        role: "reviews".into(),
    });
    e.push(MentionEntry::Skill {
        name: "review".into(),
        desc: "playbook".into(),
    });
    let got = filter(&e, "rev");
    let labels: Vec<&str> = got.iter().map(|m| m.label()).collect();
    assert_eq!(labels, vec!["reviewer", "review", "review-checklist.md"]);
}

#[test]
fn tokens_by_kind() {
    assert_eq!(
        MentionEntry::Agent {
            name: "coder".into(),
            role: String::new()
        }
        .token(),
        "coder"
    );
    assert_eq!(
        MentionEntry::Skill {
            name: "deploy".into(),
            desc: String::new()
        }
        .token(),
        "skill:deploy"
    );
    assert_eq!(
        MentionEntry::File("src/main.rs".into()).token(),
        "src/main.rs"
    );
}

#[test]
fn accept_replaces_the_trailing_token() {
    assert_eq!(accept("hey @sr", "src/main.rs"), "hey @src/main.rs ");
    assert_eq!(accept("look at @", "a.txt"), "look at @a.txt ");
}

fn tmp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("crew-mention-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn spans_cover_non_leading_at_tokens() {
    assert_eq!(spans("hey @a.rs now"), vec![(4, 9)]);
    assert_eq!(spans("@coder fix @src/x.rs"), vec![(11, 20)]); // leading selector excluded
    assert!(spans("plain text").is_empty());
    assert!(spans("hey @").is_empty()); // bare '@' is not a mention yet
}

#[test]
fn spans_cover_a_quoted_mention_as_one_chip() {
    // `see @"a b.txt" now` — the chip runs from '@' through the closing
    // quote, whitespace inside and all.
    assert_eq!(spans("see @\"a b.txt\" now"), vec![(4, 14)]);
    // An unterminated quote falls back to the whitespace-delimited token.
    assert_eq!(spans("see @\"a now"), vec![(4, 7)]);
}

#[test]
fn a_quoted_mention_round_trips_a_path_with_spaces() {
    // The exact loop a Finder drop takes: `filedrop::mention_token` mints
    // the quoted form, and `expand` must resolve it at send.
    let dir = tmp("quoted");
    std::fs::write(dir.join("my notes.txt"), "space content").unwrap();
    let tok = crate::filedrop::mention_token(&dir.join("my notes.txt"), &dir);
    assert_eq!(tok, "@\"my notes.txt\" ");
    let out = expand(&format!("summarize {tok}please"), &dir, &[]);
    assert!(
        out.contains("--- file: my notes.txt ---\nspace content\n--- end file ---"),
        "{out}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn expand_appends_mentioned_file_contents() {
    let dir = tmp("expand");
    std::fs::write(dir.join("note.txt"), "hello world").unwrap();
    let out = expand("summarize @note.txt please", &dir, &[]);
    assert!(out.starts_with("summarize @note.txt please"));
    assert!(out.contains("--- file: note.txt ---\nhello world\n--- end file ---"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// `@path:120-180` attaches those lines only. The case it exists for is
/// the file too big to attach whole — the one you most want to point an
/// agent at one function of.
#[test]
fn expand_attaches_only_the_named_lines() {
    let dir = tmp("ranges");
    let body: String = (1..=10).map(|i| format!("line{i}\n")).collect();
    std::fs::write(dir.join("f.txt"), &body).unwrap();

    let out = expand("look at @f.txt:3-5", &dir, &[]);
    assert!(out.contains("--- file: f.txt lines 3-5 ---"), "{out}");
    assert!(
        out.contains("line3\nline4\nline5\n--- end file ---"),
        "{out}"
    );
    assert!(!out.contains("line2"), "attached more than asked: {out}");
    assert!(!out.contains("line6"), "attached more than asked: {out}");

    // A single line reads as one.
    let out = expand("look at @f.txt:7", &dir, &[]);
    assert!(out.contains("--- file: f.txt line 7 ---\nline7\n"), "{out}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The cap applies to what is ATTACHED, not to the file on disk —
/// refusing forty lines because the module is 200 KB would defeat the
/// whole feature.
#[test]
fn a_range_makes_an_oversize_file_attachable() {
    let dir = tmp("range-big");
    let mut body = String::from("the interesting line\n");
    body.push_str(&"x".repeat(MAX_FILE_BYTES + 1));
    std::fs::write(dir.join("big.txt"), &body).unwrap();

    let out = expand("see @big.txt:1", &dir, &[]);
    assert!(out.contains("--- file: big.txt line 1 ---"), "{out}");
    assert!(out.contains("the interesting line"), "{out}");
    assert!(!out.contains("too large"), "{out}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A range past the end says so rather than attaching an empty block that
/// reads, to an agent, as an empty file.
#[test]
fn a_range_past_the_end_says_so() {
    let dir = tmp("range-past");
    std::fs::write(dir.join("f.txt"), "one\ntwo\n").unwrap();
    let out = expand("see @f.txt:50-60", &dir, &[]);
    assert!(out.contains("lines 50-60 is past the end"), "{out}");
    assert!(!out.contains("--- end file ---"), "{out}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A file whose NAME ends in something range-shaped is still that file.
#[test]
fn a_file_actually_named_with_a_colon_wins() {
    let dir = tmp("range-name");
    std::fs::write(dir.join("odd:10"), "the real file\n").unwrap();
    let out = expand("see @odd:10", &dir, &[]);
    assert!(out.contains("--- file: odd:10 ---"), "{out}");
    assert!(out.contains("the real file"), "{out}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The size gate runs BEFORE any read: a multi-GB drop must not be
/// slurped onto the winit thread just to be told it's too big. Pinned via
/// permissions — the file is stat-able but unreadable, so any read
/// attempt would surface "Permission denied" instead of the size note.
#[cfg(unix)]
#[test]
fn an_oversize_file_is_skipped_by_metadata_before_any_read() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tmp("statfirst");
    let path = dir.join("huge.txt");
    let f = std::fs::File::create(&path).unwrap();
    f.set_len(MAX_FILE_BYTES as u64 + 1).unwrap(); // sparse: over the cap
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o200)).unwrap();
    let out = expand("see @huge.txt", &dir, &[]);
    assert!(out.contains("huge.txt skipped: too large"), "{out}");
    assert!(
        !out.contains("skipped: Permission"),
        "the file was read before the size gate: {out}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn expand_skips_oversize_binary_and_missing() {
    let dir = tmp("caps");
    std::fs::write(dir.join("big.txt"), vec![b'a'; MAX_FILE_BYTES + 1]).unwrap();
    std::fs::write(dir.join("bin.dat"), [0u8, 159, 146, 150]).unwrap();
    let out = expand("see @big.txt @bin.dat @gone.txt", &dir, &[]);
    // The cap now names the way out: a file too big to attach whole is
    // exactly the file you want to attach part of.
    assert!(out.contains("big.txt skipped: too large"), "{out}");
    assert!(out.contains("@big.txt:<start>-<end>"), "{out}");
    assert!(out.contains("--- file: bin.dat skipped: binary ---"));
    assert!(!out.contains("gone.txt ---")); // unresolvable token left alone
    let _ = std::fs::remove_dir_all(&dir);
}

use crate::chatkeys::ChatInput;

fn open(matches: &[&str]) -> Option<MentionState> {
    Some(MentionState {
        entries: entries(matches),
        matches: entries(matches),
        sel: 0,
    })
}

#[test]
fn popup_navigates_accepts_and_closes() {
    let mut m = open(&["a.rs", "b.rs"]);
    let mut input = "see @".to_string();
    assert!(matches!(
        popup_key(&mut m, &mut input, &ChatInput::Down),
        MentionKey::Consumed
    ));
    assert_eq!(m.as_ref().unwrap().sel, 1);
    assert!(matches!(
        popup_key(&mut m, &mut input, &ChatInput::Enter),
        MentionKey::Consumed
    ));
    assert_eq!(input, "see @b.rs ");
    assert!(m.is_none()); // accept closes

    let mut m = open(&["a.rs"]);
    assert!(matches!(
        popup_key(&mut m, &mut input, &ChatInput::Close),
        MentionKey::Consumed
    ));
    assert!(m.is_none()); // Esc closes the popup, not the pane
}

#[test]
fn popup_forwards_when_closed_and_on_edits() {
    let mut m: Option<MentionState> = None;
    let mut input = String::new();
    assert!(matches!(
        popup_key(&mut m, &mut input, &ChatInput::Enter),
        MentionKey::Forward
    ));
    let mut m = open(&["a.rs"]);
    assert!(matches!(
        popup_key(&mut m, &mut input, &ChatInput::Char('x')),
        MentionKey::Forward
    ));
}

#[test]
fn after_edit_opens_refilters_and_closes() {
    let mut m: Option<MentionState> = None;
    // Typing "@" after a word opens the popup with the scanned files.
    after_edit(&mut m, "see @", || entries(&["a.rs", "b.md"]));
    assert_eq!(m.as_ref().unwrap().matches.len(), 2);
    // Narrowing the query refilters WITHOUT rescanning (scan would panic).
    after_edit(&mut m, "see @a", || unreachable!("no rescan while open"));
    assert_eq!(
        m.as_ref().unwrap().matches,
        vec![MentionEntry::File("a.rs".to_string())]
    );
    // No match → closed; token ended → stays closed.
    after_edit(&mut m, "see @zzz", || unreachable!());
    assert!(m.is_none());
    after_edit(&mut m, "see @a.rs ", || entries(&["a.rs"]));
    assert!(m.is_none());
}

#[test]
fn expand_ignores_the_leading_selector_and_dedups() {
    let dir = tmp("lead");
    std::fs::write(dir.join("a.txt"), "A").unwrap();
    // Leading token is the @agent selector even if it happens to be a path,
    // as long as it names a rostered agent.
    let out = expand("@a.txt do it", &dir, &["a.txt".to_string()]);
    assert_eq!(out, "@a.txt do it");
    let out = expand("x @a.txt and @a.txt", &dir, &[]);
    assert_eq!(out.matches("--- file: a.txt ---").count(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn expand_attaches_a_leading_non_agent_mention() {
    let dir = tmp("leadfile");
    std::fs::write(dir.join("a.txt"), "A").unwrap();
    // roster contains planner only: leading @a.txt is a mention, not routing.
    let out = expand("@a.txt summarize", &dir, &["planner".to_string()]);
    assert!(out.contains("--- file: a.txt ---"), "{out}");
    // rostered leading selector still skipped, including multi-target
    let out = expand("@planner do it @a.txt", &dir, &["planner".to_string()]);
    assert!(out.starts_with("@planner do it @a.txt"));
    assert_eq!(out.matches("--- file: a.txt ---").count(), 1);
    let out = expand("@Planner do it", &dir, &["planner".to_string()]);
    assert_eq!(out, "@Planner do it"); // roster match is case-insensitive, like broker routing
    let out = expand(
        "@planner+coder go",
        &dir,
        &["planner".to_string(), "coder".to_string()],
    );
    assert_eq!(out, "@planner+coder go");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn expand_attaches_skill_playbooks_and_leaves_unknown_skills_alone() {
    let dir = tmp("skilltok");
    let sk = dir.join(".crew/skills");
    std::fs::create_dir_all(&sk).unwrap();
    std::fs::write(sk.join("deploy.md"), "---\ndescription: d\n---\nship it").unwrap();
    let out = expand("use @skill:deploy now", &dir, &[]);
    assert!(
        out.contains("--- skill: deploy ---\nship it\n--- end skill ---"),
        "{out}"
    );
    // dedup + unknown left alone
    let out = expand("x @skill:deploy @skill:deploy @skill:ghost", &dir, &[]);
    assert_eq!(out.matches("--- skill: deploy ---").count(), 1);
    assert!(!out.contains("--- skill: ghost"));
    let _ = std::fs::remove_dir_all(&dir);
}
