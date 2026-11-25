use super::*;

#[test]
fn only_an_idle_pane_says_nothing() {
    assert_eq!(EntryState::Idle.word(), None);
    assert_eq!(EntryState::Waiting.word(), Some("you"));
    assert_eq!(EntryState::Working.word(), Some("working"));
}

#[test]
fn home_is_tilde() {
    let home = std::path::PathBuf::from(std::env::var_os("HOME").unwrap());
    assert_eq!(home_path(&home), "~");
    assert_eq!(home_path(&home.join("code/crew")), "~/code/crew");
    assert_eq!(home_path(std::path::Path::new("/etc")), "/etc");
}
