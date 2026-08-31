use super::*;

fn pool() -> Vec<String> {
    vec!["Menlo".into(), "Monaco".into(), "Hack".into()]
}

#[test]
fn pick_never_returns_the_current_family() {
    for seed in 0..50u64 {
        let p = pick(&pool(), Some("Menlo"), seed).unwrap();
        assert_ne!(p, "Menlo", "seed {seed}");
    }
}

#[test]
fn pick_is_deterministic_for_a_seed() {
    assert_eq!(
        pick(&pool(), Some("Menlo"), 7),
        pick(&pool(), Some("Menlo"), 7)
    );
}

#[test]
fn pick_returns_none_when_no_alternative_exists() {
    assert_eq!(pick(&["Menlo".to_string()], Some("Menlo"), 1), None);
    assert_eq!(pick(&[], None, 1), None);
}

#[test]
fn due_gates_on_the_shared_rotate_clock() {
    let mut r = FontRotate {
        on: true,
        last_ms: 1_000,
        ..Default::default()
    };
    assert!(!r.due(1_000 + crew_theme::ROTATE_MS - 1));
    assert!(r.due(1_000 + crew_theme::ROTATE_MS));
    r.on = false;
    assert!(!r.due(1_000 + crew_theme::ROTATE_MS));
}
