//! The section table.
use super::*;

#[test]
fn every_section_reads_back_from_its_name() {
    for s in NavSection::ALL {
        assert_eq!(NavSection::parse(s.as_str()), Some(s));
        assert_eq!(NavSection::parse(&s.as_str().to_uppercase()), Some(s));
    }
    assert_eq!(NavSection::parse("nope"), None);
}

/// An unknown or empty setting is the Panes section — the nav's navigation control.
#[test]
fn an_unknown_setting_opens_panes() {
    let mut c = crate::config::CrewConfig::default();
    assert_eq!(c.nav_section(), NavSection::Panes);
    c.nav_section = "sideways".into();
    assert_eq!(c.nav_section(), NavSection::Panes);
}
