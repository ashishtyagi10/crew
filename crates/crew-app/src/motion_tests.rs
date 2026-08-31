use super::*;

#[test]
fn every_level_round_trips() {
    for l in MotionLevel::ALL {
        assert_eq!(MotionLevel::parse(l.as_str()), Some(l));
    }
    assert_eq!(MotionLevel::parse(" FULL "), Some(MotionLevel::Full));
    assert_eq!(MotionLevel::parse("on"), Some(MotionLevel::Full));
    assert_eq!(MotionLevel::parse("swooshy"), None);
}

#[test]
fn off_collapses_every_duration() {
    assert_eq!(MotionLevel::Off.scale_ms(1_000), 0);
    assert_eq!(MotionLevel::Off.scale_ms(1), 0);
}

/// The global round-trips every level — a mis-mapped discriminant would
/// silently pin the whole app to one motion strength.
#[test]
fn the_global_round_trips_every_level() {
    let _g = crate::app::motion_test_guard();
    for l in MotionLevel::ALL {
        set_level(l);
        assert_eq!(level(), l);
    }
    set_level(MotionLevel::Full);
}

#[test]
fn scaling_is_ordered() {
    let (o, s, f) = (
        MotionLevel::Off.scale_ms(500),
        MotionLevel::Subtle.scale_ms(500),
        MotionLevel::Full.scale_ms(500),
    );
    assert!(o < s && s < f, "{o} {s} {f}");
    assert_eq!(f, 500, "Full must not stretch the nominal duration");
}

/// `auto` is a deferral, and the whole point is that it changes answer
/// when the OS switch flips — the failure mode is a `resolve` that reads
/// the flag once, or not at all, and pins auto to full motion forever.
#[test]
fn auto_follows_the_os_and_the_fixed_levels_do_not() {
    use MotionLevel::{Full, Off, Subtle};
    assert_eq!(MotionPref::Auto.resolve(false), Full);
    assert_eq!(MotionPref::Auto.resolve(true), Off);
    // An explicit choice overrules the OS in BOTH directions: a user who
    // picked `full` keeps it under Reduce Motion, and a user who picked
    // `off` does not get motion back when the switch is off.
    for l in [Off, Subtle, Full] {
        assert_eq!(MotionPref::Fixed(l).resolve(true), l);
        assert_eq!(MotionPref::Fixed(l).resolve(false), l);
    }
}

#[test]
fn every_pref_round_trips_and_auto_has_synonyms() {
    for p in MotionPref::ALL {
        assert_eq!(MotionPref::parse(p.as_str()), Some(p), "{}", p.as_str());
    }
    assert_eq!(MotionPref::parse(" AUTO "), Some(MotionPref::Auto));
    assert_eq!(MotionPref::parse("system"), Some(MotionPref::Auto));
    assert_eq!(MotionPref::parse("swooshy"), None);
}

/// The picker offers exactly what parses, and `auto` leads.
#[test]
fn the_offer_list_covers_every_level_exactly_once() {
    assert_eq!(MotionPref::ALL[0], MotionPref::Auto);
    for l in MotionLevel::ALL {
        let n = MotionPref::ALL
            .iter()
            .filter(|p| **p == MotionPref::Fixed(l))
            .count();
        assert_eq!(n, 1, "{} offered {n} times", l.as_str());
    }
}

/// `auto` alone does not tell the user whether crew is currently moving,
/// which is the only thing they came to the setting to find out.
#[test]
fn the_auto_label_names_what_it_resolved_to() {
    assert_eq!(MotionPref::Auto.label(true), "auto (off)");
    assert_eq!(MotionPref::Auto.label(false), "auto (full)");
    assert_eq!(MotionPref::Fixed(MotionLevel::Subtle).label(true), "subtle");
}

/// The OS flag is a global read by the render path; a mis-stored bit would
/// pin every `auto` user to one strength.
#[test]
fn the_os_reduce_flag_round_trips() {
    let before = os_reduce();
    set_os_reduce(true);
    assert!(os_reduce());
    set_os_reduce(false);
    assert!(!os_reduce());
    set_os_reduce(before);
}
