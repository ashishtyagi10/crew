use super::*;

#[test]
fn empty_text_has_no_suggestion() {
    assert_eq!(suggest("", &[]), None);
}

#[test]
fn slash_prefix_completes_command() {
    assert_eq!(suggest("/se", &[]).as_deref(), Some("ttings"));
    assert_eq!(suggest("/sh", &[]).as_deref(), Some("ell"));
}

#[test]
fn slash_completes_pwd() {
    assert_eq!(suggest("/pw", &[]).as_deref(), Some("d"));
}

#[test]
fn fuzzy_subsequence_finds_command() {
    // non-contiguous chars d-m-p match "/dump"
    let names: Vec<&str> = matches("/dmp").iter().map(|c| c.name).collect();
    assert!(names.contains(&"/dump"));
    // a query that isn't even a subsequence matches nothing
    assert!(matches("/zzzq").is_empty());
}

#[test]
fn prefix_matches_rank_before_fuzzy() {
    // "/se": "/settings" is a prefix match (rank 0) and must sort before
    // "/shell", which only matches as a subsequence (s…h…e…ll → rank 1).
    let names: Vec<&str> = matches("/se").iter().map(|c| c.name).collect();
    assert_eq!(names.first(), Some(&"/settings"));
    assert!(names.contains(&"/shell"));
}

#[test]
fn slash_completes_toggles() {
    assert_eq!(suggest("/broad", &[]).as_deref(), Some("cast"));
    assert_eq!(suggest("/zoo", &[]).as_deref(), Some("m"));
    assert_eq!(suggest("/side", &[]).as_deref(), Some("bar"));
}

#[test]
fn slash_completes_about() {
    assert_eq!(suggest("/ab", &[]).as_deref(), Some("out"));
    let names: Vec<&str> = matches("/ab").iter().map(|c| c.name).collect();
    assert!(names.contains(&"/about"));
}

#[test]
fn slash_completes_font() {
    assert_eq!(suggest("/fo", &[]).as_deref(), Some("nt"));
    let names: Vec<&str> = matches("/fo").iter().map(|c| c.name).collect();
    assert!(names.contains(&"/font"));
}

#[test]
fn slash_completes_reload() {
    assert_eq!(suggest("/rel", &[]).as_deref(), Some("oad"));
    let names: Vec<&str> = matches("/rel").iter().map(|c| c.name).collect();
    assert!(names.contains(&"/reload"));
}

#[test]
fn slash_completes_clearall() {
    assert_eq!(suggest("/cleara", &[]).as_deref(), Some("ll"));
    let names: Vec<&str> = matches("/cleara").iter().map(|c| c.name).collect();
    assert!(names.contains(&"/clearall"));
}

#[test]
fn slash_completes_closeall() {
    assert_eq!(suggest("/clos", &[]).as_deref(), Some("eall"));
    let names: Vec<&str> = matches("/close").iter().map(|c| c.name).collect();
    assert!(names.contains(&"/closeall"));
}

#[test]
fn slash_completes_only() {
    assert_eq!(suggest("/onl", &[]).as_deref(), Some("y"));
    let names: Vec<&str> = matches("/onl").iter().map(|c| c.name).collect();
    assert!(names.contains(&"/only"));
}

#[test]
fn slash_completes_edit() {
    assert_eq!(suggest("/ed", &[]).as_deref(), Some("it"));
    let names: Vec<&str> = matches("/ed").iter().map(|c| c.name).collect();
    assert!(names.contains(&"/edit"));
}

#[test]
fn slash_completes_run() {
    assert_eq!(suggest("/ru", &[]).as_deref(), Some("n"));
    let names: Vec<&str> = matches("/ru").iter().map(|c| c.name).collect();
    assert!(names.contains(&"/run"));
}

#[test]
fn slash_completes_copy() {
    assert_eq!(suggest("/co", &[]).as_deref(), Some("py"));
    let names: Vec<&str> = matches("/co").iter().map(|c| c.name).collect();
    assert!(names.contains(&"/copy"));
}

#[test]
fn slash_completes_dump() {
    assert_eq!(suggest("/du", &[]).as_deref(), Some("mp"));
    let names: Vec<&str> = matches("/d").iter().map(|c| c.name).collect();
    assert!(names.contains(&"/dump"));
}

#[test]
fn slash_completes_open() {
    assert_eq!(suggest("/op", &[]).as_deref(), Some("en"));
    let names: Vec<&str> = matches("/o").iter().map(|c| c.name).collect();
    assert!(names.contains(&"/open"));
}

#[test]
fn slash_completes_far() {
    assert_eq!(suggest("/fa", &[]).as_deref(), Some("r"));
    let names: Vec<&str> = matches("/f").iter().map(|c| c.name).collect();
    assert!(names.contains(&"/far") && names.contains(&"/find"));
}

#[test]
fn exact_command_offers_nothing() {
    assert_eq!(suggest("/exit", &[]), None);
}

#[test]
fn unknown_slash_has_no_suggestion() {
    assert_eq!(suggest("/zzz", &[]), None);
}

#[test]
fn history_autosuggests_most_recent_match() {
    let hist = vec!["git status".to_string(), "git push".to_string()];
    // most recent ("git push") wins for the shared "git " prefix
    assert_eq!(suggest("git ", &hist).as_deref(), Some("push"));
    assert_eq!(suggest("git s", &hist).as_deref(), Some("tatus"));
}

#[test]
fn history_no_match_is_none() {
    let hist = vec!["ls -la".to_string()];
    assert_eq!(suggest("cargo", &hist), None);
}

#[test]
fn dir_suggest_completes_subdir() {
    let base = std::env::temp_dir().join("crew_dirsuggest_test");
    std::fs::create_dir_all(base.join("alpha")).unwrap();
    std::fs::create_dir_all(base.join("beta")).unwrap();
    assert_eq!(dir_suggest("cd al", &base).as_deref(), Some("pha/"));
    assert_eq!(dir_suggest("cd be", &base).as_deref(), Some("ta/"));
    // no partial leaf, or a trailing slash → nothing to complete
    assert_eq!(dir_suggest("cd ", &base), None);
    assert_eq!(dir_suggest("cd alpha/", &base), None);
    // not a `cd` line, and a leaf that matches nothing
    assert_eq!(dir_suggest("ls al", &base), None);
    assert_eq!(dir_suggest("cd zzz", &base), None);
}

#[test]
fn matches_filters_by_prefix() {
    let names: Vec<&str> = matches("/s").iter().map(|c| c.name).collect();
    assert!(names.contains(&"/settings") && names.contains(&"/shell"));
    assert!(!names.contains(&"/exit"));
    assert!(matches("ls").is_empty()); // non-slash → no palette
}
