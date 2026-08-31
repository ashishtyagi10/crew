use super::*;

#[test]
fn now_ms_is_monotonic() {
    let a = now_ms();
    let b = now_ms();
    assert!(b >= a);
}

#[test]
fn tri_peaks_at_half_period() {
    assert_eq!(tri(0, 100), 0.0);
    assert!((tri(50, 100) - 1.0).abs() < 1e-6);
    assert!(tri(75, 100) < 0.6 && tri(75, 100) > 0.4);
    // wraps each period
    assert_eq!(tri(100, 100), 0.0);
}

#[test]
fn tri_zero_period_is_safe() {
    assert_eq!(tri(123, 0), 0.0);
}

#[test]
fn lerp_rgb_endpoints_and_midpoint() {
    let a = (0, 0, 0);
    let b = (100, 200, 50);
    assert_eq!(lerp_rgb(a, b, 0.0), a);
    assert_eq!(lerp_rgb(a, b, 1.0), b);
    assert_eq!(lerp_rgb(a, b, 0.5), (50, 100, 25));
    // clamps out-of-range t
    assert_eq!(lerp_rgb(a, b, 2.0), b);
}
