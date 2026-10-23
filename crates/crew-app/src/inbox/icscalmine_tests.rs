use super::mark_mine;
use crate::inbox::ics::reply::Answer;
use serde_json::json;

/// Your answer lands on the guest who is you, by Google's `self` or your
/// address; with no such guest nothing is marked, and the answer says so.
#[test]
fn only_a_guest_who_is_you_is_answered_and_none_is_said() {
    let mut people = vec![json!({"email": "bo@y.org"}), json!({"email": "ME@x.org"})];
    assert!(mark_mine(&mut people, "me@x.org", Answer::Yes));
    assert_eq!(people[1]["responseStatus"], "accepted");
    assert!(
        people[0].get("responseStatus").is_none(),
        "bo is left as he was"
    );
    let mut group = vec![json!({"email": "team@x.org"})];
    assert!(!mark_mine(&mut group, "me@x.org", Answer::No));
    assert!(group[0].get("responseStatus").is_none());
}
