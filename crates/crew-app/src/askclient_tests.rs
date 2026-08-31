use super::*;

#[test]
fn federate_status_reflects_the_token() {
    let on = federate_status(Some("s3cret"), "0.0.0.0", 7733);
    assert!(on.contains("ON") && on.contains("0.0.0.0:7733") && on.contains("crew://"));
    let off = federate_status(None, "0.0.0.0", 7733);
    assert!(off.contains("OFF") && off.contains("CREW_FEDERATE_TOKEN"));
    assert!(federate_status(Some(""), "0.0.0.0", 7733).contains("OFF"));
}
