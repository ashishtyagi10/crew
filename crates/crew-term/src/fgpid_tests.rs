//! The foreground answer when the OS can't give one.
use super::settle;

const SHELL: Option<u32> = Some(100);

#[test]
fn the_shell_leading_its_group_is_the_shell_back() {
    assert_eq!(settle(Some(100), SHELL, Some(200)), None);
    assert_eq!(settle(Some(200), SHELL, None), Some(200));
}

/// A failed lookup while Claude Code runs is not the shell back: the
/// running program stays the answer, so no edge fires the leftover reset.
#[test]
fn a_failed_lookup_keeps_the_last_answer() {
    assert_eq!(settle(None, SHELL, Some(200)), Some(200));
    assert_eq!(settle(None, SHELL, None), None);
}
