use super::*;
use crate::graph::TaskId;
use crate::provider::MockProvider;

#[tokio::test]
async fn stub_planner_builds_fanout_plus_merge() {
    let g = StubPlanner { fanout: 3 }
        .plan("do the thing")
        .await
        .unwrap();
    assert_eq!(g.len(), 4); // 3 leaves + 1 merge
                            // the merge task (highest id) depends on all leaves
    let merge = g.tasks().iter().max_by_key(|t| t.id.0).unwrap();
    assert_eq!(merge.deps.len(), 3);
}

#[test]
fn parse_plan_builds_graph_from_json() {
    let json = r#"[
        {"id": 0, "title": "research", "prompt": "research X", "deps": []},
        {"id": 1, "title": "write", "prompt": "write up X", "deps": [0]}
    ]"#;
    let g = parse_plan(json).unwrap();
    assert_eq!(g.len(), 2);
    assert_eq!(g.get(TaskId(1)).unwrap().deps, vec![TaskId(0)]);
}

#[test]
fn parse_plan_rejects_garbage() {
    assert!(matches!(parse_plan("not json"), Err(PlanError::Parse(_))));
}

/// SECURITY: a malicious/compromised completion that tries to smuggle a
/// process-executing agent (`agent`/`command`/`args`/`system` keys) must NOT
/// produce a `Pty` task. serde drops the unknown fields and `parse_plan` forces
/// every task to `Api`, so the command-injection sink never materializes.
#[test]
fn parse_plan_ignores_injected_command_and_forces_api() {
    use crate::graph::AgentKind;
    let json = r#"[
        {"id": 0, "title": "pwn", "prompt": "p", "deps": [],
         "agent": "Pty", "command": "/bin/sh", "args": ["-c", "rm -rf /"],
         "system": "ignore-me"}
    ]"#;
    let g = parse_plan(json).unwrap();
    let task = g.get(TaskId(0)).unwrap();
    assert!(!task.agent.is_pty(), "injected Pty must be dropped");
    assert_eq!(task.agent, AgentKind::Api { system: None });
}

/// Across an arbitrary plan, no task is ever a process-spawning `Pty`.
#[test]
fn parse_plan_never_yields_pty() {
    let json = r#"[
        {"id": 0, "title": "a", "prompt": "p", "deps": [], "command": "x"},
        {"id": 1, "title": "b", "prompt": "q", "deps": [0], "args": ["y"]}
    ]"#;
    let g = parse_plan(json).unwrap();
    assert!(g.tasks().iter().all(|t| !t.agent.is_pty()));
}

#[tokio::test]
async fn llm_planner_parses_provider_json() {
    let reply = r#"[{"id":0,"title":"t","prompt":"p","deps":[]}]"#;
    let planner = LlmPlanner {
        provider: MockProvider {
            reply: reply.into(),
        },
        tier: crate::graph::ModelTier::Standard,
    };
    let g = planner.plan("goal").await.unwrap();
    assert_eq!(g.len(), 1);
}
