use super::*;

#[test]
fn palette_hides_shell_and_run_but_dispatch_keeps_them() {
    assert!(
        !COMMANDS
            .iter()
            .any(|c| c.name == "/shell" || c.name == "/run"),
        "bare text replaced these palette rows"
    );
}

#[test]
fn empty_text_has_no_suggestion() {
    assert_eq!(suggest("", &[]), None);
}

#[test]
fn slash_prefix_completes_command() {
    assert_eq!(suggest("/se", &[]).as_deref(), Some("ttings"));
    assert_eq!(suggest("/sm", &[]).as_deref(), Some("ith"));
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
    // "/sidebar", which only matches as a subsequence (s…e…b → rank 1).
    let names: Vec<&str> = matches("/se").iter().map(|c| c.name).collect();
    assert_eq!(names.first(), Some(&"/settings"));
    assert!(names.contains(&"/sidebar"));
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
fn slash_completes_restore_and_restart_is_gone() {
    // /restart merged into /update, so /res now belongs to /restore.
    assert_eq!(suggest("/res", &[]).as_deref(), Some("tore"));
    let names: Vec<&str> = matches("/res").iter().map(|c| c.name).collect();
    assert!(names.contains(&"/restore"));
    assert!(
        !crate::suggest::COMMANDS
            .iter()
            .any(|c| c.name == "/restart"),
        "/restart should be dropped from the palette"
    );
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
fn theme_space_lists_the_rotations_first_then_every_palette() {
    let items = menu_items("/theme ");
    let labels: Vec<&str> = items.iter().map(|m| m.label.as_str()).collect();
    // The four rotations lead — they are what most people want — then a
    // heading, then every individual palette. The palettes have always
    // parsed; not offering them meant you had to know the name already.
    assert_eq!(
        &labels[..4],
        &["dark", "light", "crt", "auto"],
        "{labels:?}"
    );
    assert!(items[4].header, "the palettes need a heading: {labels:?}");
    for name in ["paper-dark", "crt-green", "crt-violet", "harbor", "fern"] {
        assert!(labels.contains(&name), "{name} is missing: {labels:?}");
    }
    // The legacy rotation names still parse and are still not offered.
    for name in ["modern", "modern-light", "random-dark"] {
        assert!(!labels.contains(&name), "{name} must not be a picker entry");
    }
    // Every offered row runs on Enter, and a heading is not a row.
    for item in &items {
        assert_eq!(item.submit, !item.header, "{}", item.label);
    }
}

#[test]
fn model_command_expands_into_a_value_picker() {
    // Typing "/model" offers the command row, filling a trailing space so the
    // value picker opens rather than running.
    let model = menu_items("/model")
        .into_iter()
        .find(|m| m.label == "/model")
        .expect("/model in palette");
    assert_eq!(model.fill, "/model ");
    assert!(!model.submit, "the command expands into its picker");
}

#[test]
fn model_space_lists_runnable_model_values() {
    let items = menu_items("/model ");
    let labels: Vec<&str> = items.iter().map(|m| m.label.as_str()).collect();
    assert!(
        labels.contains(&"default"),
        "picker offers default: {labels:?}"
    );
    assert!(
        labels.contains(&"qwen-max"),
        "picker offers qwen-max: {labels:?}"
    );
    // Each value fills the full command and submits on Enter.
    let qwen = items.iter().find(|m| m.label == "qwen-max").unwrap();
    assert_eq!(qwen.fill, "/model qwen-max");
    assert!(qwen.submit);
}

#[test]
fn model_picker_filters_by_partial_value() {
    let items = menu_items("/model claude");
    assert!(!items.is_empty(), "claude-* models match");
    assert!(
        items.iter().all(|m| m.label.starts_with("claude")),
        "only claude models survive the filter: {:?}",
        items.iter().map(|m| &m.label).collect::<Vec<_>>()
    );
}

#[test]
fn theme_picker_describes_each_mode() {
    let items = menu_items("/theme ");
    for (mode, desc) in [
        ("dark", "rotating dark pages \u{2014} paper and modern glow"),
        (
            "light",
            "rotating light pages \u{2014} paper and modern glow",
        ),
        ("crt", "rotating CRT phosphor themes"),
    ] {
        let item = items
            .iter()
            .find(|m| m.label == mode)
            .unwrap_or_else(|| panic!("{mode} not found in picker"));
        assert_eq!(item.fill, format!("/theme {mode}"));
        assert!(item.submit);
        assert_eq!(item.desc, desc);
    }
}

#[test]
fn theme_partial_value_filters_and_ghosts() {
    // "/theme cr" narrows to the crt mode…
    let labels: Vec<String> = menu_items("/theme cr")
        .into_iter()
        .map(|m| m.label)
        .collect();
    // …to the crt rotation and, under the heading, the tubes themselves.
    assert_eq!(labels[0], "crt", "picker: {labels:?}");
    assert!(labels.contains(&"crt-violet".to_string()), "{labels:?}");
    assert!(!labels.contains(&"dark".to_string()), "{labels:?}");
    // …and ghost-completes it like a command.
    assert_eq!(suggest("/theme cr", &[]).as_deref(), Some("t"));
}

#[test]
fn freeform_command_has_no_picker() {
    // /run takes an arbitrary command, so past its space there's no value list.
    assert!(menu_items("/run cargo build").is_empty());
}

#[test]
fn agent_cli_aliases_removed_in_favor_of_run() {
    // The /claude, /codex, /opencode aliases were dropped — bare text input
    // can force a new pane (e.g. `claude` or `codex`), and /smith (formerly
    // /crew, still a dispatch alias) still opens the multi-agent relay.
    let all: Vec<&str> = matches("/").iter().map(|c| c.name).collect();
    for name in ["/claude", "/codex", "/opencode"] {
        assert!(
            !all.contains(&name),
            "{name} should no longer be in the palette"
        );
    }
    assert!(all.contains(&"/smith"));
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
    assert!(names.contains(&"/settings") && names.contains(&"/sidebar"));
    assert!(!names.contains(&"/exit"));
    assert!(matches("ls").is_empty()); // non-slash → no palette
}

#[test]
fn slash_completes_restore_from_its_short_prefix() {
    assert_eq!(suggest("/resto", &[]).as_deref(), Some("re"));
    let names: Vec<&str> = matches("/res").iter().map(|c| c.name).collect();
    assert!(names.contains(&"/restore"));
}

#[test]
fn step_sel_skips_header_rows_in_both_directions() {
    use crate::suggest::{first_selectable, step_sel, MenuItem};
    fn row(label: &str, header: bool) -> MenuItem {
        MenuItem {
            label: label.to_string(),
            desc: String::new(),
            fill: label.to_string(),
            submit: false,
            header,
            dim: false,
            needs: None,
            color: None,
            ..Default::default()
        }
    }
    // [hdr, a, b, hdr, c]
    let items = vec![
        row("anthropic", true),
        row("a", false),
        row("b", false),
        row("openai", true),
        row("c", false),
    ];
    assert_eq!(first_selectable(&items), 1); // never lands on a header
    assert_eq!(step_sel(&items, 1, true), 2);
    assert_eq!(step_sel(&items, 2, true), 4); // hops the header at 3
    assert_eq!(step_sel(&items, 4, true), 1); // wraps past the leading header
    assert_eq!(step_sel(&items, 1, false), 4); // wraps backwards
    assert_eq!(step_sel(&items, 4, false), 2);
    // All-headers can't move anywhere: stay put rather than spin.
    let only = vec![row("anthropic", true)];
    assert_eq!(step_sel(&only, 0, true), 0);
    assert_eq!(first_selectable(&only), 0);
}

/// The palette's whole point after this change: among commands that match a
/// query equally well, the ones the user actually runs come first. Without it
/// the order is `cmddefs`'s declaration order, which means something to
/// whoever last edited that file and nothing to the person typing.
#[test]
fn recently_run_commands_lead_among_equal_matches() {
    // A bare `/` matches everything as a prefix, so nothing but the tie-break
    // is deciding the order here.
    let cold: Vec<&str> = matches_with("/", &[]).iter().map(|c| c.name).collect();
    let warm_list = vec!["/gradient".to_string(), "/density".to_string()];
    let warm: Vec<&str> = matches_with("/", &warm_list)
        .iter()
        .map(|c| c.name)
        .collect();

    assert_eq!(&warm[..2], &["/gradient", "/density"], "recents lead");
    assert_ne!(cold[0], warm[0], "the list must actually reorder");
    // Nothing is lost or duplicated — this is a sort, not a filter.
    let mut a = cold.clone();
    let mut b = warm.clone();
    a.sort_unstable();
    b.sort_unstable();
    assert_eq!(a, b);
}

/// The rule that keeps a learned list predictable: recency reorders WITHIN a
/// match-quality band and never across one. If a much-used command could jump
/// the prefix matches on a fuzzy score, the palette would stop being something
/// you can aim at — you would have to read it every time.
#[test]
fn recency_never_promotes_a_fuzzy_match_over_a_prefix_match() {
    // `/de` is a prefix of /density and a subsequence of several others.
    let prefixed: Vec<&str> = matches_with("/de", &[])
        .iter()
        .map(|c| c.name)
        .filter(|n| n[1..].starts_with("de"))
        .collect();
    assert!(!prefixed.is_empty(), "the fixture needs a prefix match");

    // Make every NON-prefix match maximally recent.
    let hot: Vec<String> = matches_with("/de", &[])
        .iter()
        .map(|c| c.name.to_string())
        .filter(|n| !n[1..].starts_with("de"))
        .collect();
    assert!(!hot.is_empty(), "the fixture needs a fuzzy match too");

    let got: Vec<&str> = matches_with("/de", &hot).iter().map(|c| c.name).collect();
    for (i, name) in got.iter().enumerate() {
        if !name[1..].starts_with("de") {
            // Everything before the first fuzzy match must be a prefix match.
            assert_eq!(
                i,
                prefixed.len(),
                "a fuzzy match reached position {i} ahead of a prefix match: {got:?}"
            );
            break;
        }
    }
}

/// A prefix query marks the run it matched; the slash is never part of it.
#[test]
fn a_prefix_query_marks_the_characters_it_matched() {
    assert_eq!(super::hit_positions("/dump", "du"), vec![1, 2]);
    assert_eq!(super::hit_positions("/dump", ""), Vec::<usize>::new());
}

/// A fuzzy hit marks the letters that actually matched, which is the only
/// thing that explains why `/dmp` found `/dump`.
#[test]
fn a_fuzzy_query_marks_the_scattered_letters() {
    assert_eq!(super::hit_positions("/dump", "dmp"), vec![1, 3, 4]);
    assert_eq!(super::hit_positions("/findall", "fall"), vec![1, 5, 6, 7]);
}

/// Leftmost-greedy, and case-insensitive the way the matcher is: `/Dump`
/// typed in capitals still marks the same characters.
#[test]
fn matching_is_case_insensitive_and_takes_the_leftmost_letters() {
    assert_eq!(super::hit_positions("/dump", "DU"), vec![1, 2]);
    // "mm" in "/comm": both m's, not the same one twice.
    assert_eq!(super::hit_positions("/comm", "mm"), vec![3, 4]);
}

/// A query that runs out of label marks what it got and stops — never a
/// position past the end, which the row would then try to draw.
#[test]
fn a_query_longer_than_the_label_marks_only_what_matched() {
    let hit = super::hit_positions("/x", "xyz");
    assert_eq!(hit, vec![1]);
    assert!(hit.iter().all(|&i| i < "/x".chars().count()));
}

/// End to end: the palette's rows carry both the marks and the chord.
#[test]
fn palette_rows_carry_their_marks_and_their_chord() {
    let items = super::menu_items("/clear");
    let row = items
        .iter()
        .find(|i| i.label == "/clear")
        .expect("/clear is in the palette");
    assert_eq!(row.hit, vec![1, 2, 3, 4, 5]);
    assert_eq!(row.key, Some("Cmd+K"));
    let plain = items
        .iter()
        .find(|i| i.label != "/clear" && i.key.is_none())
        .map(|i| i.key);
    assert_eq!(plain.flatten(), None, "a chordless command shows no chord");
}

/// A heading with nothing under it is a lie about where you are in the list —
/// the same rule the `/keys` filter follows.
#[test]
fn a_heading_with_no_palettes_under_it_is_dropped() {
    // "au" matches the `auto` rotation and no palette name.
    let items = menu_items("/theme au");
    let labels: Vec<&str> = items.iter().map(|m| m.label.as_str()).collect();
    assert_eq!(labels, vec!["auto"], "{labels:?}");
    assert!(!items.iter().any(|i| i.header), "an empty heading survived");
}
