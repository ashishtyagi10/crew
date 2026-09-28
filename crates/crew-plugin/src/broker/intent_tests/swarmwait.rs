//! Agent smith is visibly at work while the swarm plans.
use super::*;

fn smith_at(evs: &[PluginEvent], want: &str, after: usize) -> Option<usize> {
    evs.iter()
        .enumerate()
        .skip(after)
        .find_map(|(i, e)| match e {
            PluginEvent::Activity { agent, state, .. }
                if agent == "agent smith" && state == want =>
            {
                Some(i)
            }
            _ => None,
        })
}

#[test]
fn smith_thinks_through_the_planning_and_settles_as_the_plan_lands() {
    let _g = testenv::mock("ok");
    let call = |_: &str| Ok("SHAPE: swarm".to_string());
    let evs = route_stubbed("build the thing in parts", &call);
    let routed = evs
        .iter()
        .position(|e| text_of(e).starts_with("routing: swarm"))
        .expect("routed");
    let plan = evs
        .iter()
        .position(|e| matches!(e, PluginEvent::HivePlan { .. }))
        .expect("planned");
    let thinking = smith_at(&evs, "thinking", routed).expect("thinking while planning");
    let idle = smith_at(&evs, "idle", thinking).expect("settled");
    assert!(
        routed < thinking && thinking < idle && idle < plan,
        "{evs:?}"
    );
}
