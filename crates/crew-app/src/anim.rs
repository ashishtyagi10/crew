//! Shared animation clock + tiny easing helpers. Time-based UI animations read
//! [`now_ms`] so they share one timeline. The clock is just elapsed wall time;
//! animations only drive redraws while they're actually active (see the busy
//! handling in `poll`), so an idle Crew never repaints — animation never costs
//! performance when nothing is moving.
use std::sync::OnceLock;
use std::time::Instant;

/// Milliseconds since the first call — the shared animation timeline.
pub fn now_ms() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

/// Triangle wave in `0.0..=1.0`: ramps 0→1 over the first half of `period_ms`,
/// then 1→0 over the second half. Drives a value that bounces back and forth
/// (e.g. an indeterminate bar's sweep position).
pub fn tri(now: u64, period_ms: u64) -> f32 {
    if period_ms == 0 {
        return 0.0;
    }
    let p = (now % period_ms) as f32 / period_ms as f32; // 0..1
    if p < 0.5 {
        p * 2.0
    } else {
        2.0 - p * 2.0
    }
}

/// Linear blend of two RGB colours by `t` (clamped to `0.0..=1.0`).
pub fn lerp_rgb(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    (mix(a.0, b.0), mix(a.1, b.1), mix(a.2, b.2))
}

#[cfg(test)]
mod tests {
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
}
