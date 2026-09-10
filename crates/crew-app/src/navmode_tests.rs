use super::*;

#[test]
fn nav_card_names_round_trip_and_reject_the_rest() {
    for c in NavCard::ALL {
        assert_eq!(NavCard::parse(c.as_str()), Some(c));
    }
    assert_eq!(NavCard::parse(" LOG "), Some(NavCard::Log));
    assert_eq!(NavCard::parse("weather"), None);
}
