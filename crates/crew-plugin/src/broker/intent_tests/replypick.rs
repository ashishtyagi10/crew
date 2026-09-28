//! A reply is answered by the agent the router names for it, not by whoever
//! answered last — and an `@name` the user typed still outranks the router.
use super::*;

fn dialled(evs: &[PluginEvent]) -> Vec<String> {
    evs.iter()
        .filter_map(|e| match e {
            PluginEvent::Activity { agent, state, .. }
                if state == "thinking" && agent != "agent smith" =>
            {
                Some(agent.clone())
            }
            _ => None,
        })
        .collect()
}

#[test]
fn the_router_s_pick_answers_the_reply() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let call = |_: &str| Ok("SHAPE: reply\nAGENTS: reviewer".to_string());
    let evs = route_stubbed("is this diff safe to merge?", &call);
    assert_eq!(
        dialled(&evs).first().map(String::as_str),
        Some("reviewer"),
        "{evs:?}"
    );
    assert_eq!(routing_line_of(&evs), "routing: reply \u{2192} reviewer");
}

#[test]
fn a_pick_the_roster_lacks_falls_back_to_the_default() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let call = |_: &str| Ok("SHAPE: reply\nAGENTS: nobody".to_string());
    let evs = route_stubbed("hello", &call);
    assert_eq!(
        dialled(&evs).first().map(String::as_str),
        Some("planner"),
        "{evs:?}"
    );
}

#[test]
fn the_world_names_each_agent_with_its_role() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let w = crate::broker::intent::world::World::gather(&Session::new());
    let s = w.section();
    assert!(s.contains("reviewer ("), "{s}");
}

fn routing_line_of(evs: &[PluginEvent]) -> String {
    evs.iter()
        .map(text_of)
        .find(|t| t.starts_with("routing: "))
        .unwrap_or_default()
        .split(" \u{b7} ")
        .next()
        .unwrap_or_default()
        .to_string()
}
