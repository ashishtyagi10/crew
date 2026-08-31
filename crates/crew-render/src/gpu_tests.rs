//! What each window picks for itself, and what the process picks once.
use wgpu::TextureFormat as F;

use super::{pick_alpha_mode, pick_surface_format};

#[test]
fn prefers_a_non_srgb_format_for_gamma_space_blending() {
    // Whatever order the platform lists them, non-sRGB wins.
    assert_eq!(
        pick_surface_format(&[F::Bgra8UnormSrgb, F::Bgra8Unorm]),
        F::Bgra8Unorm
    );
    assert_eq!(
        pick_surface_format(&[F::Bgra8Unorm, F::Bgra8UnormSrgb]),
        F::Bgra8Unorm
    );
}

#[test]
fn falls_back_to_the_first_format_when_all_are_srgb() {
    assert_eq!(
        pick_surface_format(&[F::Bgra8UnormSrgb, F::Rgba8UnormSrgb]),
        F::Bgra8UnormSrgb
    );
}

mod alpha {
    use wgpu::CompositeAlphaMode as M;

    use super::pick_alpha_mode;

    /// Our shaders write straight alpha, so PostMultiplied is the mode that
    /// composites a translucent window correctly.
    #[test]
    fn prefers_post_multiplied() {
        assert_eq!(
            pick_alpha_mode(&[M::Opaque, M::PostMultiplied]),
            M::PostMultiplied
        );
        assert_eq!(
            pick_alpha_mode(&[M::PostMultiplied, M::PreMultiplied, M::Opaque]),
            M::PostMultiplied
        );
    }

    #[test]
    fn falls_back_to_premultiplied_then_to_whatever_exists() {
        assert_eq!(
            pick_alpha_mode(&[M::Opaque, M::PreMultiplied]),
            M::PreMultiplied
        );
        // An Opaque-only platform still has to produce a working surface —
        // the window simply cannot go translucent there.
        assert_eq!(pick_alpha_mode(&[M::Opaque]), M::Opaque);
        assert_eq!(pick_alpha_mode(&[]), M::Auto);
    }
}

/// The whole point of the split: the second window through here gets the
/// device the first one made, not one of its own.
///
/// Asserted on the handle itself — resources belong to the device that made
/// them, so two devices can share nothing at all, and a count of how many
/// were *asked for* would still pass if each ask built one.
#[test]
#[ignore = "needs a GPU adapter"]
fn every_window_gets_the_same_device() {
    let Ok(first) = super::shared_for(None) else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    let second = super::shared_for(None).expect("a second window");
    let third = super::shared_for(None).expect("a third");
    assert!(
        std::sync::Arc::ptr_eq(&first, &second) && std::sync::Arc::ptr_eq(&first, &third),
        "a window built its own device instead of taking the one that exists"
    );
}
