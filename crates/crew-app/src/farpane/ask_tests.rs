use super::*;

#[test]
fn bang_ask_parses_the_description() {
    assert_eq!(bang_ask("! list rust files"), Some("list rust files"));
    assert_eq!(bang_ask("!  kill port 8080 "), Some("kill port 8080"));
    assert_eq!(bang_ask("!"), Some(""));
    assert_eq!(bang_ask("!   "), Some(""));
}

#[test]
fn lines_without_a_leading_bang_are_not_an_ask() {
    assert_eq!(bang_ask("ls -la"), None);
    assert_eq!(bang_ask("echo hi!"), None);
    assert_eq!(bang_ask(""), None);
}
