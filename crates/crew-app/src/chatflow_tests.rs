fn test_pane() -> crate::chat::ChatPane {
    // An idle child stands in for the broker; only pane state is under test.
    let plugin =
        crew_plugin::Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()])
            .unwrap();
    crate::chat::ChatPane::new(plugin, "crew".into())
}

#[test]
fn turn_total_accumulates_split_and_cost() {
    let mut pane = test_pane();
    pane.absorb_stats(950, String::new(), 0, 0, 900, 50, 129);
    pane.absorb_stats(100, String::new(), 0, 0, 90, 10, 21);
    assert_eq!(pane.tok_in, 990);
    assert_eq!(pane.tok_out, 60);
    assert_eq!(pane.cost_microusd, 150);
    // Per-agent reply stats must NOT double-count into session totals.
    pane.absorb_stats(500, "coder".into(), 800, 400, 450, 50, 60);
    assert_eq!(pane.tok_in, 990);
    assert_eq!(pane.tok_out, 60);
    assert_eq!(pane.cost_microusd, 150);
}
