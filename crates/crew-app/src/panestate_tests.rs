use super::*;

#[test]
fn only_an_idle_pane_says_nothing() {
    assert_eq!(EntryState::Idle.word(), None);
    assert_eq!(EntryState::Waiting.word(), Some("you"));
    assert_eq!(EntryState::Working.word(), Some("working"));
}
