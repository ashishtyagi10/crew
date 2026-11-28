use super::*;
use crew_hive::HiveEvent;

fn call(label: &str) -> PluginEvent {
    PluginEvent::Hive {
        event: HiveEvent::ToolCall {
            agent: crew_hive::AgentId::minted("claude"),
            label: label.into(),
            args: "{}".into(),
        },
    }
}

fn thought() -> PluginEvent {
    PluginEvent::Thought {
        agent: "claude".into(),
        text: "hm".into(),
    }
}

#[test]
fn tools_classify_into_the_four_kinds() {
    use crate::chatphase::of_tool;
    for (tool, k) in [
        ("Read", Kind::Read),
        ("sys:grep", Kind::Read),
        ("sys:read_file", Kind::Read),
        ("mcp__x__ls", Kind::Read),
        ("Edit", Kind::Edit),
        ("sys:write_file", Kind::Edit),
        ("Bash", Kind::Run),
        ("sys:run", Kind::Run),
        ("mystery", Kind::Think),
        ("Task", Kind::Think),
    ] {
        assert_eq!(Kind::of(of_tool(tool)), k, "{tool}");
    }
}

#[test]
fn the_clock_gives_each_phase_the_time_it_held() {
    let mut c = Clock::start(0);
    c.hear(&call("Read"), 4_000); // 4 s thinking
    c.hear(&call("Bash"), 6_000); // 2 s reading
    c.hear(&thought(), 16_000); // 10 s running
    let got = c.snapshot(20_000).unwrap(); // 4 s more thinking
    assert_eq!(got, [8_000, 2_000, 0, 10_000]);
}

#[test]
fn zero_tools_is_all_think_and_a_short_turn_is_none() {
    let c = Clock::start(100);
    assert_eq!(c.snapshot(30_100), Some([30_000, 0, 0, 0]));
    assert_eq!(c.snapshot(600), None);
    assert_eq!(Clock::default().snapshot(99_000), None);
}

#[test]
fn the_stamp_round_trips_and_leaves_the_task_tag_in_front() {
    let m = stamp("task:7", [1, 2, 3, 4]);
    assert_eq!(m, "task:7 \u{b7} strip:1,2,3,4");
    assert_eq!(read(&m), Some([1, 2, 3, 4]));
    assert_eq!(crate::chattime::task_tag(&m), Some(7));
    assert_eq!(crate::chattime::strip_task_tag(&m), "");
    assert_eq!(read("task:7"), None);
    assert_eq!(read("strip:1,2,3"), None);
    assert_eq!(unstamped("4.2s"), "4.2s");
}

#[test]
fn a_stopped_clock_does_not_grow() {
    let mut c = Clock::start(0);
    c.stop(30_000);
    assert_eq!(
        c.snapshot(600_000),
        c.snapshot(30_000),
        "ten minutes later, the same"
    );
    assert_eq!(c.snapshot(600_000).unwrap().iter().sum::<u64>(), 30_000);
}
