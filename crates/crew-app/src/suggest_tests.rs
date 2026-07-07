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
fn slash_completes_font() {
    assert_eq!(suggest("/fo", &[]).as_deref(), Some("nt"));
    let names: Vec<&str> = matches("/fo").iter().map(|c| c.name).collect();
    assert!(names.contains(&"/font"));
}

#[test]
fn slash_completes_restart() {
    assert_eq!(suggest("/res", &[]).as_deref(), Some("tart"));
    let names: Vec<&str> = matches("/res").iter().map(|c| c.name).collect();
    assert!(names.contains(&"/restart"));
}

#[test]
fn slash_completes_only() {
    assert_eq!(suggest("/onl", &[]).as_deref(), Some("y"));
    let names: Vec<&str> = matches("/onl").iter().map(|c| c.name).collect();
    assert!(names.contains(&"/only"));
}

#[test]
fn edit_and_open_were_dropped_for_far() {
    // File operations live in Far (F3/F4/Enter) and Cmd+click now; the old
    // /edit and /open input-bar commands are gone from the palette.
    for name in ["/edit", "/open"] {
        assert!(
            !crate::suggest::COMMANDS.iter().any(|c| c.name == name),
            "{name} should be dropped"
        );
    }
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
fn theme_command_expands_into_a_value_picker() {
    // Typing the command name: the /theme row fills "/theme " (a trailing space)
    // and does NOT submit — selecting it opens the value list.
    let theme = menu_items("/theme")
        .into_iter()
        .find(|m| m.label == "/theme")
        .expect("/theme in palette");
    assert_eq!(theme.fill, "/theme ");
    assert!(!theme.submit, "picker command expands, doesn't run");
}

#[test]
fn theme_space_lists_all_themes_as_runnable_values() {
    let items = menu_items("/theme ");
    let labels: Vec<&str> = items.iter().map(|m| m.label.as_str()).collect();
    for name in [
        "paper-dark",
        "paper-light",
        "crt-green",
        "crt-amber",
        "crt-blue",
    ] {
        assert!(labels.contains(&name), "{name} missing from picker");
    }
    // Each value fills the full command and submits on Enter.
    let green = items.iter().find(|m| m.label == "crt-green").unwrap();
    assert_eq!(green.fill, "/theme crt-green");
    assert!(green.submit);
}

#[test]
fn theme_picker_offers_random_mode() {
    // `random` is a valid /theme value, so the picker must offer it alongside
    // the fixed themes — typing it blind shouldn't be the only way in.
    let items = menu_items("/theme ");
    let random = items
        .iter()
        .find(|m| m.label == "random")
        .expect("random missing from picker");
    assert_eq!(random.fill, "/theme random");
    assert!(random.submit);
    // And it survives partial-value filtering.
    let labels: Vec<String> = menu_items("/theme ra")
        .into_iter()
        .map(|m| m.label)
        .collect();
    assert_eq!(labels, ["random"]);
}

#[test]
fn theme_partial_value_filters_and_ghosts() {
    // "/theme cr" narrows to the CRT themes…
    let labels: Vec<String> = menu_items("/theme cr")
        .into_iter()
        .map(|m| m.label)
        .collect();
    assert!(labels.iter().all(|l| l.starts_with("cr")));
    assert!(labels.contains(&"crt-green".to_string()));
    // …and ghost-completes the first match like a command.
    assert_eq!(suggest("/theme crt-a", &[]).as_deref(), Some("mber"));
}

#[test]
fn freeform_command_has_no_picker() {
    // /run takes an arbitrary command, so past its space there's no value list.
    assert!(menu_items("/run cargo build").is_empty());
}

#[test]
fn agent_cli_aliases_removed_in_favor_of_run() {
    // The /claude, /codex, /opencode aliases were dropped — `/run <tool>` covers
    // them (and /crew still opens the multi-agent relay).
    let all: Vec<&str> = matches("/").iter().map(|c| c.name).collect();
    for name in ["/claude", "/codex", "/opencode"] {
        assert!(
            !all.contains(&name),
            "{name} should no longer be in the palette"
        );
    }
    assert!(all.contains(&"/run"));
    assert!(all.contains(&"/crew"));
}

#[test]
fn slash_completes_clearlog() {
    // /clear is the shortest match, so it ghosts first; /clearlog is reached by
    // typing past it, and both appear in the palette.
    assert_eq!(suggest("/clearl", &[]).as_deref(), Some("og"));
    let names: Vec<&str> = matches("/clear").iter().map(|c| c.name).collect();
    assert!(names.contains(&"/clear") && names.contains(&"/clearlog"));
}

#[test]
fn slash_completes_dump() {
    assert_eq!(suggest("/du", &[]).as_deref(), Some("mp"));
    let names: Vec<&str> = matches("/d").iter().map(|c| c.name).collect();
    assert!(names.contains(&"/dump"));
}

#[test]
fn slash_completes_far() {
    assert_eq!(suggest("/fa", &[]).as_deref(), Some("r"));
    let names: Vec<&str> = matches("/f").iter().map(|c| c.name).collect();
    assert!(names.contains(&"/far") && names.contains(&"/find"));
}

#[test]
fn slash_completes_md() {
    assert_eq!(suggest("/m", &[]).as_deref(), Some("d"));
    let names: Vec<&str> = matches("/m").iter().map(|c| c.name).collect();
    assert!(names.contains(&"/md"));
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
