//! Prompt regression for `PLANNER_SYSTEM`, against a real provider.
//!
//! `#[ignore]`d: needs `DASHSCOPE_API_KEY` and spends tokens. Run with
//! `cargo test -p crew-hive --test planner_prompt -- --ignored --nocapture`.
//!
//! Asserts only the mechanical properties (slug-legality without mangling,
//! distinctness, no title echo) and prints the cast for eyeballing — "is this
//! a good name" is a human call. See the design doc's Prompt spike section
//! for what each prompt clause defends against.
use crew_hive::agentname::slug;
use crew_hive::{LlmPlanner, ModelTier, OpenRouterProvider, Planner};

const ENDPOINT: &str = "https://dashscope-intl.aliyuncs.com/compatible-mode/v1/chat/completions";

/// Deliberately non-coding: the roster is meant to be a network of diverse
/// specialists, not a coding crew, so the prompt is judged on breadth.
const GOALS: &[&str] = &[
    "explain our project to stakeholders",
    "audit our dependencies for CVEs",
    "plan a 3-day trip to Kyoto in November",
    "write a blog post announcing our new release",
    "figure out why checkout conversion dropped 12% last month",
    "design a schema for a multi-tenant billing system",
];

#[test]
#[ignore = "network + API key + tokens"]
fn planner_invents_craft_shaped_specialists() {
    let key = std::env::var("DASHSCOPE_API_KEY").expect("DASHSCOPE_API_KEY");
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut total = 0usize;
    let mut distinct = std::collections::HashSet::new();

    for goal in GOALS {
        let provider = OpenRouterProvider::new(key.clone())
            .with_endpoint(ENDPOINT.to_string())
            .with_fallbacks(vec!["qwen-max".to_string()]);
        let planner = LlmPlanner {
            provider,
            tier: ModelTier::Capable,
            model: Some("qwen-max".to_string()),
        };
        let graph = rt.block_on(planner.plan(goal)).expect("plan");

        println!("\n=== {goal}");
        let mut seen_in_plan = std::collections::HashSet::new();
        for t in graph.tasks() {
            println!("  {:<28} | {:<45} | {}", t.specialty, t.title, t.expertise);
            total += 1;
            distinct.insert(t.specialty.clone());

            assert_eq!(
                slug(&t.specialty).as_deref(),
                Some(t.specialty.as_str()),
                "specialty {:?} is not slug-stable",
                t.specialty
            );
            assert!(
                !t.specialty.starts_with("specialist-"),
                "task {:?} fell back to a derived name — the model omitted or \
                 mangled its specialty",
                t.title
            );
            // The failure signature this guards: the model echoing the task
            // title back instead of naming a craft. Never observed on
            // qwen-max across ~150 names, but a different model might.
            assert_ne!(
                Some(t.specialty.as_str()),
                slug(&t.title).as_deref(),
                "specialty {:?} is just the task title slugged",
                t.specialty
            );
            seen_in_plan.insert(t.specialty.clone());
        }
        assert!(
            seen_in_plan.len() > 1,
            "a whole plan collapsed to one specialist: {seen_in_plan:?}"
        );
    }

    // The spike measured 28 distinct / 32. A hard floor here would be flaky;
    // this catches only a collapse to near-uniformity.
    println!("\n{} distinct / {} total", distinct.len(), total);
    assert!(
        distinct.len() * 2 > total,
        "specialists are barely distinct ({} distinct / {total}) — the prompt \
         has probably regressed toward a catch-all",
        distinct.len()
    );
}

/// The scheduler runs every ready task at once (`JoinSet` + a semaphore), so
/// the concurrency a run actually achieves is decided HERE — by the width of
/// the graph the planner returns. A chain of `deps: [N-1]` executes serially
/// no matter what the cap is.
///
/// `PLANNER_SYSTEM` gives `deps` one passive clause and never says independent
/// work should stay independent, so this is the property most at risk from an
/// innocuous prompt edit — and nothing else asserts graph shape.
#[test]
#[ignore = "network + API key + tokens"]
fn planner_leaves_independent_work_independent() {
    let key = std::env::var("DASHSCOPE_API_KEY").expect("DASHSCOPE_API_KEY");
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut wide = 0usize;

    for goal in GOALS {
        let provider = OpenRouterProvider::new(key.clone())
            .with_endpoint(ENDPOINT.to_string())
            .with_fallbacks(vec!["qwen-max".to_string()]);
        let planner = LlmPlanner {
            provider,
            tier: ModelTier::Capable,
            model: Some("qwen-max".to_string()),
        };
        let graph = rt.block_on(planner.plan(goal)).expect("plan");

        // Replay the scheduler's own loop: repeatedly take everything `ready`,
        // mark it done, and record the widest wave. That IS the peak
        // concurrency the scheduler would reach, so measuring it here needs no
        // agents, no runtime and no cap.
        let mut done: std::collections::HashSet<_> = std::collections::HashSet::new();
        let mut widest = 0usize;
        let mut waves: Vec<usize> = Vec::new();
        while done.len() < graph.len() {
            let wave = graph.ready(&done);
            assert!(!wave.is_empty(), "{goal}: graph stalled — cycle?");
            widest = widest.max(wave.len());
            waves.push(wave.len());
            for id in wave {
                done.insert(id);
            }
        }
        println!("{widest:>2}-wide  waves={waves:?}  {goal}");
        if widest >= 2 {
            wide += 1;
        }
    }

    // Not every goal SHOULD be wide — "audit our dependencies for CVEs" is
    // honestly serial (list → scan → prioritise → report, each needing the
    // last), and forcing width there would be a worse plan, not a better one.
    // So this is a floor on the set, not on each goal: it catches the observed
    // failure — the model chaining tasks that have no real dependency — while
    // leaving it free to serialise work that genuinely is.
    //
    // Sampled before the `deps must be MINIMAL` clause: one run had 5/6 wide,
    // a later run collapsed "explain our project to stakeholders" to a flat
    // 6-task chain. The shape is non-deterministic run to run, which is why
    // this floor sits well below the observed best rather than at it.
    println!("\n{wide}/{} goals have parallel width", GOALS.len());
    assert!(
        wide * 2 >= GOALS.len(),
        "only {wide}/{} goals left any independent work independent — the \
         planner is chaining tasks that could run at once, and the scheduler \
         cannot recover the lost concurrency whatever its cap",
        GOALS.len()
    );
}
