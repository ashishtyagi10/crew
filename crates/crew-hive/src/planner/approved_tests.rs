//! The plan gate's contract with the planner: an approved plan is the spec.
//! Source-pinned, as the capabilities test pins the prompt: the sentence
//! lives in `PLANNER_SYSTEM`, so the bare prompt (no capabilities) carries it
//! and the goal that names an `APPROVED PLAN` section meets a planner that
//! was told what that section means.
use super::*;
use crate::provider::MockProvider;

#[test]
fn the_planner_is_told_an_approved_plan_is_the_task_breakdown() {
    assert!(
        PLANNER_SYSTEM.contains("APPROVED PLAN"),
        "the clause names the section the plan gate frames"
    );
    assert!(
        PLANNER_SYSTEM.contains("one-to-one"),
        "one task per step, no re-decomposition"
    );
    assert!(
        PLANNER_SYSTEM.contains("unless the plan says steps are independent"),
        "deps follow the step order by default"
    );
    let bare = LlmPlanner {
        provider: MockProvider { reply: "[]".into() },
        tier: crate::graph::ModelTier::Standard,
        model: None,
        capabilities: Vec::new(),
    };
    assert!(
        bare.system().contains("APPROVED PLAN"),
        "and it is in the prompt the provider is sent"
    );
}
