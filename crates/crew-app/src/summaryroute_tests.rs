use super::*;
use crate::chatsummary::footer_lines_with;
use crate::glyphs::force;
use crew_plugin::AgentInfo;
use crew_theme::contrast_ratio;
use std::collections::HashMap;

fn readouts() -> &'static crate::readout::Readouts {
    Box::leak(Box::default())
}

fn fc<'a>(agents: &'a [AgentInfo], ctx: &'a HashMap<String, u64>) -> FooterCtx<'a> {
    FooterCtx {
        agents,
        ctx,
        tok_in: 0,
        tok_out: 0,
        cost_microusd: 0,
        branch: None,
        input: "",
        running_tasks: &[],
        plan_pending: false,
        active: Vec::new(),
        cwd: None,
        windows: crate::usageledger::Windows::default(),
        readouts: readouts(),
        pulse: None,
    }
}

fn text(line: &[FCell]) -> String {
    line.iter().map(|c| c.0).collect()
}

/// The run of cells carrying `label`, by position.
fn run<'a>(line: &'a [FCell], label: &str) -> &'a [FCell] {
    let chars: Vec<char> = line.iter().map(|c| c.0).collect();
    let want: Vec<char> = label.chars().collect();
    let at = (0..chars.len())
        .find(|&i| chars[i..].starts_with(&want))
        .unwrap_or_else(|| panic!("{label:?} not in {:?}", text(line)));
    &line[at..at + want.len()]
}

/// On main the mode was plain yellow text and the names plain text in
/// their colours. They are badges now: the mode on the accent block, each
/// name on its roster colour — the block, not the ink, is what says who —
/// capped both ends, and the ink on every block clears the text floor.
#[test]
fn the_mode_and_the_working_agents_are_badges_on_their_colours() {
    let _g = crate::app::theme_test_guard();
    let _off = force(false);
    let ctx = HashMap::new();
    let mut f = fc(&[], &ctx);
    f.active = vec!["analyst", "coder"];
    f.running_tasks = &[3];
    let l3 = &footer_lines_with(&f, 120, &mut Vec::new())[2];
    let s = text(l3);
    assert!(
        s.starts_with("\u{2590} \u{25b6}\u{25b6} swarm mode \u{258c} \u{00b7} \u{2590} @analyst \u{258c} \u{00b7} \u{2590} @coder \u{258c} \u{00b7} running #3"),
        "{s:?}"
    );
    let accent = crate::palette::accent();
    let floor = crew_theme::contrast::text_floor();
    for (label, block) in [
        ("swarm mode", accent),
        ("@analyst", crate::chatroster::agent_color("analyst")),
        ("@coder", crate::chatroster::agent_color("coder")),
    ] {
        for cell in run(l3, label) {
            assert_eq!(cell.2, Some(block), "{label} {:?} on its block", cell.0);
            assert!(
                contrast_ratio(cell.1, block) >= floor,
                "{label}: {:?} on {block:?} = {:.2}",
                cell.1,
                contrast_ratio(cell.1, block)
            );
        }
    }
    // The caps wear the block on the page; the tail sits on the page.
    let cap = l3[0];
    assert_eq!((cap.1, cap.2), (accent, None));
    assert!(run(l3, "running #3").iter().all(|c| c.2.is_none()));
}

/// The names take their caps into the width budget: every line fits, the
/// names go first, and at 40 columns the mode and the work ids both stand
/// — capped, whole, and never a badge cut in half.
#[test]
fn a_forty_column_pane_keeps_the_mode_badge_whole_and_drops_the_names() {
    let _g = crate::app::theme_test_guard();
    let _off = force(false);
    let ctx = HashMap::new();
    for cols in [24usize, 30, 40, 60, 80, 120] {
        let mut f = fc(&[], &ctx);
        f.active = vec!["analyst", "coder"];
        f.running_tasks = &[3];
        let l3 = &footer_lines_with(&f, cols, &mut Vec::new())[2];
        let s = text(l3);
        let w: usize = l3.iter().map(|c| crate::chatwidth::char_w(c.0)).sum();
        assert!(w <= cols, "{cols}: {w} wide: {s:?}");
        let caps = |ch: char| s.chars().filter(|&c| c == ch).count();
        assert_eq!(
            caps('\u{2590}'),
            caps('\u{258c}'),
            "{cols}: a lone cap: {s:?}"
        );
        assert!(s.contains("#3"), "{cols}: work ids lost: {s:?}");
        if cols == 40 {
            assert_eq!(
                s,
                "\u{2590} \u{25b6}\u{25b6} swarm mode \u{258c} \u{00b7} running #3"
            );
        }
        if cols <= 30 {
            assert!(!s.contains('@'), "{cols}: names survived: {s:?}");
        }
    }
}

/// The relay mode names its target inside the badge; on a Nerd Font the
/// caps are the powerline arcs.
#[test]
fn a_relay_target_is_named_in_the_badge_with_powerline_caps_when_on() {
    let _g = crate::app::theme_test_guard();
    let _on = force(true);
    let agents = [AgentInfo {
        name: "coder".into(),
        role: String::new(),
        model: "m".into(),
    }];
    let ctx = HashMap::new();
    let mut f = fc(&agents, &ctx);
    f.input = "@coder fix the tests";
    let l3 = &footer_lines_with(&f, 120, &mut Vec::new())[2];
    assert!(
        text(l3).starts_with("\u{e0b6} \u{25b6}\u{25b6} @coder relay \u{e0b4} \u{00b7} "),
        "{:?}",
        text(l3)
    );
}
