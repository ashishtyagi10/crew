use super::*;

#[test]
fn linear_conversion_endpoints_and_monotonic() {
    assert_eq!(srgb_channel_to_linear(0), 0.0);
    assert!((srgb_channel_to_linear(255) - 1.0).abs() < 1e-6);
    // 8/255 sRGB is ~0.0024 linear — the near-black page must stay near black.
    let low = srgb_channel_to_linear(8);
    assert!(low < 0.004, "got {low}");
    let mut prev = -1.0;
    for c in 0..=255u8 {
        let v = srgb_channel_to_linear(c);
        assert!(v > prev);
        prev = v;
    }
}

#[test]
fn target_rgba_respects_format() {
    let srgb = target_rgba((128, 128, 128), 1.0, true);
    let raw = target_rgba((128, 128, 128), 1.0, false);
    assert!(srgb[0] < raw[0], "linear value sits below the raw fraction");
    assert_eq!(raw[0], 128.0 / 255.0);
    assert_eq!(srgb[3], 1.0);
}
