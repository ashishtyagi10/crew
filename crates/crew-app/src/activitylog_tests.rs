use super::*;

#[test]
fn errors_lead_with_a_greppable_tag_and_info_stays_bare() {
    assert_eq!(
        render(LogLevel::Error, "12:01 broker died"),
        "ERR 12:01 broker died"
    );
    assert_eq!(
        render(LogLevel::Info, "12:01 copied 3 lines"),
        "12:01 copied 3 lines"
    );
}

#[test]
fn the_log_lives_next_to_the_config() {
    let p = path().expect("config dir");
    assert!(p.ends_with("crew/activity.log"), "{p:?}");
}
