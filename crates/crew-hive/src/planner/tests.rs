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
