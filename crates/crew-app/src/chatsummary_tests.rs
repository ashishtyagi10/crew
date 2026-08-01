use super::*;
use crew_plugin::AgentInfo;
use std::collections::HashMap;

fn agent(name: &str, model: &str) -> AgentInfo {
    AgentInfo {
        name: name.into(),
        role: String::new(),
        model: model.into(),
    }
}

fn ctx(pairs: &[(&str, u64)]) -> HashMap<String, u64> {
    pairs.iter().map(|(n, v)| (n.to_string(), *v)).collect()
}

fn text(line: &[(char, (u8, u8, u8))]) -> String {
    line.iter().map(|(c, _)| *c).collect()
}

/// A fresh set of counters per call, so each case sees its numbers settled on
/// first sight rather than mid-sweep from the previous test's values.
fn readouts() -> &'static crate::readout::Readouts {
    Box::leak(Box::new(crate::readout::Readouts::default()))
}

fn fc<'a>(agents: &'a [AgentInfo], ctxm: &'a HashMap<String, u64>) -> FooterCtx<'a> {
    FooterCtx {
        readouts: readouts(),
        agents,
        ctx: ctxm,
        tok_in: 41_600,
        tok_out: 314,
        cost_microusd: 129_000, // $0.129
        branch: Some("main"),
        input: "",
        running_tasks: &[],
        plan_pending: false,
        cwd: None,
        windows: crate::usageledger::Windows {
            five_h: Some(crate::usageledger::WindowStat {
                left_ms: (3 * 60 + 52) * 60_000, // 3h52m
                spent: 150_000,
                budget: 5_000_000, // 3%
            }),
            seven_d: Some(crate::usageledger::WindowStat {
                left_ms: (3 * 24 + 23) * 3_600_000, // 3d23h
                spent: 0,
                budget: 25_000_000,
            }),
        },
    }
}

#[test]
fn line1_shows_model_branch_cost_and_split() {
    let agents = [agent("smith", "qwen/qwen3-coder-plus")];
    let lines = footer_lines(&fc(&agents, &ctx(&[("smith", 100_000)])), 120);
    assert_eq!(
        text(&lines[0]),
        "qwen3-coder-plus | main | $0.129 | 41.6k in / 314 out"
    );
}

#[test]
fn line1_hides_cost_when_unpriced_but_always_shows_tokens() {
    let empty_ctx = HashMap::new();
    let mut f = fc(&[], &empty_ctx);
    f.cost_microusd = 0;
    f.tok_in = 0;
    f.tok_out = 0;
    f.branch = None;
    let lines = footer_lines(&f, 120);
    assert_eq!(text(&lines[0]), "0 in / 0 out");
}

#[test]
fn line2_shows_countdowns_and_bars() {
    let agents = [agent("smith", "anthropic/claude-opus-4-8")];
    // opus limit 200k, 100k used → ctx bar 50%.
    let lines = footer_lines(&fc(&agents, &ctx(&[("smith", 100_000)])), 120);
    let l2 = text(&lines[1]);
    assert!(l2.starts_with("5h:3h52m | 7d:3d23h | "), "{l2}");
    assert!(l2.contains("3% (5h)"), "{l2}");
    assert!(l2.ends_with("50% (ctx)"), "{l2}");
}

#[test]
fn line2_dashes_when_no_window_and_drops_ctx_without_agents() {
    let empty_ctx = HashMap::new();
    let mut f = fc(&[], &empty_ctx);
    f.windows = crate::usageledger::Windows::default();
    let l2 = text(&footer_lines(&f, 120)[1]);
    assert_eq!(l2, "5h:-- | 7d:--");
}

#[test]
fn line2_drops_bars_on_narrow_panes() {
    let agents = [agent("smith", "anthropic/claude-opus-4-8")];
    let lines = footer_lines(&fc(&agents, &ctx(&[("smith", 100_000)])), 40);
    assert_eq!(text(&lines[1]), "5h:3h52m | 7d:3d23h");
}

#[test]
fn line3_swarm_by_default_relay_when_mentioning() {
    let agents = [agent("coder", "m")];
    let empty_ctx = HashMap::new();
    let f = fc(&agents, &empty_ctx);
    let l3 = text(&footer_lines(&f, 120)[2]);
    assert_eq!(
        l3,
        "\u{25b6}\u{25b6} swarm mode \u{00b7} / for constructs \u{00b7} @ to relay to an agent"
    );
    let mut f = fc(&agents, &empty_ctx);
    f.input = "@coder fix the tests";
    let l3 = text(&footer_lines(&f, 120)[2]);
    assert!(l3.starts_with("\u{25b6}\u{25b6} @coder relay"), "{l3}");
}

#[test]
fn segments_are_colored_separators_muted() {
    let agents = [agent("smith", "qwen3-coder-plus")];
    let lines = footer_lines(&fc(&agents, &ctx(&[])), 120);
    let th = crew_theme::theme();
    // First char of line 1 is the model segment → cyan (ansi[14]).
    assert_eq!(lines[0][0].1, th.ansi[14]);
    // The separator chars are muted.
    let sep = lines[0].iter().find(|(c, _)| *c == '|').unwrap();
    assert_eq!(sep.1, th.text_muted);
}

/// Agents on different models are named rather than counted: the name is what
/// you type to reach one, and two names cost less width than "2 agents".
#[test]
fn a_mixed_roster_names_its_agents() {
    let agents = [agent("a", "m1"), agent("b", "m2")];
    let lines = footer_lines(&fc(&agents, &HashMap::new()), 120);
    assert!(
        text(&lines[0]).starts_with("a\u{00b7}b | "),
        "{}",
        text(&lines[0])
    );
}

/// The keyless roster: external CLI agents report NO model, because the CLI
/// picks it. The old model-only rendering drew a bare separator with nothing
/// before it for exactly this case.
#[test]
fn a_modelless_cli_roster_shows_names_not_a_gap() {
    let agents = [
        agent("claude", ""),
        agent("codex", ""),
        agent("opencode", ""),
    ];
    let line = text(&footer_lines(&fc(&agents, &HashMap::new()), 120)[0]);
    assert!(
        line.starts_with("claude\u{00b7}codex\u{00b7}opencode | "),
        "{line}"
    );
    assert!(!line.starts_with(" |"), "empty leading segment: {line}");
}

/// One agent with a model keeps naming the MODEL — that is the identity worth
/// the width when there is only one thing to route to.
#[test]
fn a_single_modelled_agent_still_shows_its_model() {
    let agents = [agent("smith", "anthropic/claude-opus-4-8")];
    let line = text(&footer_lines(&fc(&agents, &HashMap::new()), 120)[0]);
    assert!(line.starts_with("claude-opus-4-8 | "), "{line}");
}

/// Past a few, names stop fitting and stop informing.
#[test]
fn a_crowded_roster_falls_back_to_a_count() {
    let agents = [
        agent("a", ""),
        agent("b", ""),
        agent("c", ""),
        agent("d", ""),
    ];
    let line = text(&footer_lines(&fc(&agents, &HashMap::new()), 120)[0]);
    assert!(line.starts_with("4 agents | "), "{line}");
}

/// API specialists share one model; CLI agents (empty model) ride along. The
/// model is the identity the user asked the line for — the count still says
/// how many hands are on deck.
#[test]
fn a_mixed_cli_and_api_roster_leads_with_the_shared_model() {
    let agents = [
        agent("planner", "qwen/qwen3-coder-plus"),
        agent("coder", "qwen/qwen3-coder-plus"),
        agent("claude", ""),
    ];
    let line = text(&footer_lines(&fc(&agents, &HashMap::new()), 120)[0]);
    assert!(
        line.starts_with("qwen3-coder-plus \u{00b7} 3 agents | "),
        "{line}"
    );
}

/// A large all-API roster also shows the model, not just a count.
#[test]
fn a_crowded_agreeing_roster_keeps_the_model_with_its_count() {
    let agents = [
        agent("a", "m1"),
        agent("b", "m1"),
        agent("c", "m1"),
        agent("d", "m1"),
    ];
    let line = text(&footer_lines(&fc(&agents, &HashMap::new()), 120)[0]);
    assert!(line.starts_with("m1 \u{00b7} 4 agents | "), "{line}");
}

/// An empty roster contributes no segment at all — not an empty one.
#[test]
fn an_empty_roster_contributes_nothing() {
    let line = text(&footer_lines(&fc(&[], &HashMap::new()), 120)[0]);
    assert!(!line.starts_with(" |") && !line.starts_with("|"), "{line}");
}

/// Work in flight outranks the hints: the hints teach someone deciding what to
/// type, and this line is what `/tasks` used to have to be asked for.
#[test]
fn running_tasks_replace_the_hints_on_line3() {
    let empty_ctx = HashMap::new();
    let mut f = fc(&[], &empty_ctx);
    f.running_tasks = &[3, 5];
    let l3 = text(&footer_lines(&f, 120)[2]);
    assert!(l3.contains("running #3 #5"), "{l3}");
    // Ids are shown because `/stop #n` needs one to name.
    assert!(l3.contains("/stop #3 to cancel"), "{l3}");
    assert!(!l3.contains("for constructs"), "hints survived: {l3}");
}

#[test]
fn an_idle_pane_keeps_its_hints() {
    let empty_ctx = HashMap::new();
    let f = fc(&[], &empty_ctx);
    let l3 = text(&footer_lines(&f, 120)[2]);
    assert!(l3.contains("for constructs"), "{l3}");
    assert!(!l3.contains("running"), "{l3}");
}

/// Past a few, the ids stop fitting and the count is the information.
#[test]
fn a_crowd_of_tasks_collapses_to_a_count() {
    let empty_ctx = HashMap::new();
    let mut f = fc(&[], &empty_ctx);
    f.running_tasks = &[1, 2, 3, 4];
    let l3 = text(&footer_lines(&f, 120)[2]);
    assert!(l3.ends_with("4 running"), "{l3}");
}

/// The directory leads line 1 — it answers "where will this actually run",
/// which is the question `/cwd` existed to ask.
#[test]
fn the_working_directory_leads_line1() {
    let empty_ctx = HashMap::new();
    let mut f = fc(&[], &empty_ctx);
    f.cwd = Some("~/code/crew");
    let l1 = text(&footer_lines(&f, 120)[0]);
    assert!(l1.starts_with("~/code/crew | "), "{l1}");
}

/// First to go when width is short: it is the most stable value on the line,
/// so it is the least costly to lose.
#[test]
fn a_narrow_pane_drops_the_directory_first() {
    let empty_ctx = HashMap::new();
    let mut f = fc(&[], &empty_ctx);
    f.cwd = Some("~/code/crew");
    let l1 = text(&footer_lines(&f, 40)[0]);
    assert!(!l1.contains("~/code/crew"), "{l1}");
    assert!(l1.contains("in / "), "the rest of the line survived: {l1}");
}

/// A pane whose directory is not known yet contributes no segment — not an
/// empty one, and not a stray separator.
#[test]
fn an_unknown_directory_contributes_nothing() {
    let empty_ctx = HashMap::new();
    let mut f = fc(&[], &empty_ctx);
    f.cwd = Some("");
    let l1 = text(&footer_lines(&f, 120)[0]);
    assert!(!l1.starts_with(" |") && !l1.starts_with("|"), "{l1}");
}

/// A pending plan is a question addressed to the user, so it outranks both
/// the hints and the running-task list on line 3 — nothing else there is
/// waiting on an answer.
#[test]
fn a_pending_plan_owns_line3() {
    let empty_ctx = HashMap::new();
    let mut f = fc(&[], &empty_ctx);
    f.plan_pending = true;
    f.running_tasks = &[7];
    let l3 = text(&footer_lines(&f, 120)[2]);
    assert!(l3.contains("plan ready"), "{l3}");
    assert!(l3.contains("enter runs it"), "{l3}");
    assert!(l3.contains("esc discards it"), "{l3}");
    assert!(
        !l3.contains("running #7"),
        "tasks outranked the question: {l3}"
    );
}

/// The footer must FIT. `place_row` clips whatever overruns, silently and
/// from the right, so an unbudgeted line lost the spend meter on a narrow
/// pane while keeping the branch name — purely because of where each sat.
#[test]
fn every_footer_line_fits_its_pane() {
    let agents = [
        agent("claude", ""),
        agent("codex", ""),
        agent("opencode", ""),
    ];
    let ctxm = ctx(&[("claude", 100_000)]);
    for cols in [24usize, 32, 40, 60, 72, 80, 100, 120] {
        let mut f = fc(&agents, &ctxm);
        f.cwd = Some("~/some/deep/project/path");
        for (i, line) in footer_lines(&f, cols).iter().enumerate() {
            let w = text(line).chars().count();
            // Line 3 is prose and is clipped by design; lines 1 and 2 are
            // segment lists and must budget themselves.
            if i < 2 {
                assert!(
                    w <= cols,
                    "line {} is {w} wide in {cols} cols: {}",
                    i + 1,
                    text(line)
                );
            }
        }
    }
}

/// What survives when it cannot all fit: who is answering outranks what it
/// cost, which outranks where and on what branch.
#[test]
fn the_roster_is_the_last_thing_to_go() {
    let agents = [agent("smith", "qwen3-coder-plus")];
    let ctxm = HashMap::new();
    let mut f = fc(&agents, &ctxm);
    f.cwd = Some("~/code/crew");
    // Wide: everything shows.
    let wide = text(&footer_lines(&f, 120)[0]);
    assert!(wide.contains("~/code/crew") && wide.contains("main") && wide.contains("in /"));
    // Narrow: the directory goes first, the branch next, the model last.
    let narrow = text(&footer_lines(&f, 44)[0]);
    assert!(
        !narrow.contains("~/code/crew"),
        "cwd survived 44 cols: {narrow}"
    );
    assert!(
        narrow.contains("qwen3-coder-plus"),
        "identity lost first: {narrow}"
    );
    let tiny = text(&footer_lines(&f, 20)[0]);
    assert!(
        tiny.contains("qwen3-coder-plus"),
        "identity lost at 20: {tiny}"
    );
    assert!(tiny.chars().count() <= 20, "{tiny}");
}

/// Line 3 budgets too. It used to be one string, clipped — so a 40-column
/// pane showed "enter runs it" and cut "esc discards it", teaching how to
/// accept a plan and not how to decline it. Half an instruction is worse
/// than none.
#[test]
fn line3_fits_and_never_teaches_half_an_answer() {
    let agents = [agent("coder", "m")];
    let ctxm = HashMap::new();
    for cols in [24usize, 30, 40, 60, 80, 120] {
        // Idle: the mode outranks the hints.
        let f = fc(&agents, &ctxm);
        let idle = text(&footer_lines(&f, cols)[2]);
        assert!(idle.chars().count() <= cols, "idle {cols}: {idle}");

        // A pending plan: both keys survive together or neither does.
        let mut g = fc(&agents, &ctxm);
        g.plan_pending = true;
        let plan = text(&footer_lines(&g, cols)[2]);
        assert!(plan.chars().count() <= cols, "plan {cols}: {plan}");
        assert!(plan.contains("plan ready"), "plan {cols}: {plan}");
        let has_accept = plan.contains("enter");
        let has_decline = plan.contains("esc");
        assert_eq!(
            has_accept, has_decline,
            "{cols} cols taught half the answer: {plan}"
        );

        // Running: the ids outrank the way to cancel them.
        let mut h = fc(&agents, &ctxm);
        h.running_tasks = &[3, 5];
        let busy = text(&footer_lines(&h, cols)[2]);
        assert!(busy.chars().count() <= cols, "busy {cols}: {busy}");
        assert!(busy.contains("running #3 #5"), "busy {cols}: {busy}");
    }
}
