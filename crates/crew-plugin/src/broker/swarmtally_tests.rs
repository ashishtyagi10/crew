use super::*;

fn budget(used: u32, total: u32) -> HiveEvent {
    HiveEvent::ToolBudget { used, total }
}

/// The first empty reading says it, a refused draw's included, and no later
/// one repeats it.
#[test]
fn the_empty_reading_says_it_once() {
    let mut t = Tally::new(3);
    assert!(t.observe(&budget(11, 12)).is_none());
    let note = t.observe(&budget(12, 12)).expect("the empty pool says it");
    assert!(
        matches!(note, PluginEvent::Message { ref text, .. } if text.starts_with("tool budget spent \u{2014} 12 rounds"))
    );
    assert!(t.observe(&budget(12, 12)).is_none(), "said once");
}
