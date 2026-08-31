use super::*;

#[test]
fn envelope_new_starts_at_hop_zero() {
    let e = Envelope::new("user", "claude", "t1", "hi");
    assert_eq!(
        (e.from.as_str(), e.to.as_str(), e.hop),
        ("user", "claude", 0)
    );
    assert_eq!(e.thread_id, "t1");
    assert_eq!(e.body, "hi");
}
