use super::*;

#[test]
fn fraction_zero_total() {
    assert_eq!(fraction(0, 0), 0.0);
}

#[test]
fn fraction_half() {
    assert_eq!(fraction(50, 100), 0.5);
}

#[test]
fn fraction_clamps_over_total() {
    assert_eq!(fraction(200, 100), 1.0);
}

#[test]
fn stats_default() {
    assert_eq!(
        Stats::default(),
        Stats {
            cpu: 0.0,
            mem: 0.0,
            disk: 0.0,
            ..Default::default()
        }
    );
}

#[test]
fn sampler_new_ranges() {
    let s = SysSampler::new().stats();
    assert!((0.0..=1.0).contains(&s.cpu));
    assert!((0.0..=1.0).contains(&s.mem));
    assert!((0.0..=1.0).contains(&s.disk));
}
