//! Real `sh`, real processes: what each test starts it must also end, since a
//! leaked `sleep` outlives the suite (`Drop for SysBg` is the backstop).
use super::*;
use std::time::Duration;

const SEC: Duration = Duration::from_secs(5);

/// Start `cmd`, which must print its background child's pid on one line.
fn with_child(bg: &SysBg, cmd: &str) -> (Arc<Proc>, String) {
    let p = bg.spawn(cmd).unwrap();
    assert_eq!(p.out.wait(Some("\n"), SEC), Waited::Found);
    let pid = p.out.read().0.trim().to_string();
    (p, pid)
}

fn alive(pid: &str) -> bool {
    Command::new("/bin/kill")
        .args(["-0", pid])
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn gone(pid: &str) -> bool {
    (0..60).any(|_| {
        std::thread::sleep(Duration::from_millis(50));
        !alive(pid)
    })
}

const PARENT: &str = "sleep 300 & echo $!; wait";

#[test]
fn kill_ends_the_whole_group_and_says_how() {
    let bg = SysBg::default();
    let (p, pid) = with_child(&bg, PARENT);
    assert!(alive(&pid), "the grandchild runs while the task does");
    let said = bg.kill(p.id).unwrap();
    assert!(said.contains("signal") || said.contains("status"), "{said}");
    assert!(gone(&pid), "a child of the child must die with the group");
    assert!(!p.out.running());
}

#[test]
fn a_fifth_live_process_is_refused_by_name() {
    let bg = SysBg::default();
    let ids: Vec<u32> = (0..MAX_LIVE)
        .map(|_| bg.spawn("sleep 300").unwrap().id)
        .collect();
    let why = bg.spawn("sleep 300").err().unwrap();
    assert!(why.contains("#1") && why.contains("#4"), "{why}");
    bg.kill(ids[0]).unwrap();
    assert!(bg.spawn("sleep 300").is_ok(), "a freed slot is usable");
}

#[test]
fn the_task_ending_reaps_everything_it_started() {
    let bg = SysBg::default();
    let (a, pa) = with_child(&bg, PARENT);
    let (_, pb) = with_child(&bg, PARENT);
    bg.reap();
    assert!(gone(&pa) && gone(&pb), "reap must leave nothing running");
    assert!(bg.get(a.id).is_err(), "and nothing registered");
}

#[test]
fn a_process_that_exits_on_its_own_reports_its_status_and_output() {
    let bg = SysBg::default();
    let p = bg.spawn("echo hi; echo oops >&2; exit 3").unwrap();
    assert_eq!(p.out.wait(None, SEC), Waited::Exited);
    let (text, _, exit) = p.out.read();
    assert_eq!(exit.as_deref(), Some("exited with status 3"));
    assert!(
        text.contains("hi") && text.contains("oops"),
        "both streams: {text}"
    );
    assert!(!p.out.running());
}

#[test]
fn the_ring_keeps_only_the_latest_bytes() {
    let bg = SysBg::with_cap(1000);
    let p = bg
        .spawn("yes abcdefghi | head -c 100000; echo END")
        .unwrap();
    assert_eq!(p.out.wait(None, SEC), Waited::Exited);
    let (text, dropped, _) = p.out.read();
    assert!(text.len() <= 1000, "{} bytes kept", text.len());
    assert!(dropped >= 99_000, "{dropped} dropped");
    assert!(text.trim_end().ends_with("END"), "the newest survives");
}

#[test]
fn kill_also_reaches_what_a_finished_shell_left_behind() {
    let bg = SysBg::default();
    // `sh` exits at once; the orphan it backgrounded ignores TERM, so only the final KILL ends it.
    let (p, pid) = with_child(&bg, "trap '' TERM; sleep 300 & echo $!");
    assert_eq!(p.out.wait(None, SEC), Waited::Exited);
    assert!(alive(&pid));
    bg.kill(p.id).unwrap();
    assert!(gone(&pid), "the orphan must not survive sys:kill");
}
