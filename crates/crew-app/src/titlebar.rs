//! The title bar of a sheer window is never see-through.
//!
//! The title bar is the OS's, not crew's: AppKit draws it, and it composites
//! against `NSWindow.isOpaque`. A translucent crew window has to be
//! non-opaque, and a non-opaque window's title bar shows the desktop through
//! it — which is backwards: the chrome that names the window went glassy
//! while the panes were meant to.
//!
//! So while the window is sheer, the titlebar container view gets a SOLID
//! layer background in the page colour, beneath the traffic lights and the
//! title. At full opacity the layer background is cleared and the native bar
//! is untouched. Crew's frame never reaches this strip, so there is nothing to
//! solidify on the GPU side.
//!
//! The bar's INK comes with it: the container also wears the page's own
//! appearance (Aqua on a light page, Dark Aqua on a dark one). Without it the
//! title and traffic lights followed the OS — a dark Mac under a light crew
//! (auto's daylight hours, or a pinned light theme) drew a white title on the
//! light strip crew had just painted. Set on the container, not the window,
//! so the window's own appearance — the one winit reports as the OS theme,
//! which `auto` follows — stays the system's.
use winit::window::Window;

/// What the title bar wears while crew owns it: the page colour, and whether
/// that page is dark (so its title is drawn light). `None` is the OS's bar.
pub type Wear = Option<([u8; 3], bool)>;

impl crate::app::CrewApp {
    /// Keep the title bar solid while the window is sheer (see
    /// [`crate::titlebar`]). Run every frame, but it only reaches AppKit when
    /// the wanted colour moves — an opacity change or a theme switch, from
    /// whichever of the several paths switched it.
    pub(crate) fn sync_titlebar(&mut self) {
        let t = crew_theme::theme();
        let want = crate::titlebar::wanted(self.config.window_opacity, t.page_bg, t.dark);
        if want == self.titlebar_paint {
            return;
        }
        if let Some(w) = &self.window {
            crate::titlebar::paint(w, want);
            self.titlebar_paint = want;
        }
    }
}

/// Paint the title bar solid in the page colour, with ink for that page, or
/// hand it back to the OS with `None`.
/// Idempotent and cheap to repeat, but the caller only calls it on a change
/// (see `CrewApp::sync_titlebar`).
#[cfg(target_os = "macos")]
pub fn paint(window: &Window, wear: Wear) {
    use objc2::encode::{Encoding, RefEncode};
    use objc2::runtime::{AnyObject, Bool};
    use objc2::{class, msg_send};
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

    /// Opaque `CGColor`, encoded the way AppKit's method signatures name it
    /// so objc2's debug encoding check accepts the pointer.
    #[repr(C)]
    struct CGColor {
        _p: [u8; 0],
    }
    unsafe impl RefEncode for CGColor {
        const ENCODING_REF: Encoding = Encoding::Pointer(&Encoding::Struct("CGColor", &[]));
    }

    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::AppKit(h) = handle.as_raw() else {
        return;
    };
    // SAFETY: winit hands out a live NSView for as long as `window` lives, we
    // are on the main thread (the winit thread), and every message below is
    // public AppKit API on the object it is sent to. The one private detail
    // is the container's CLASS NAME, used only to find it; if AppKit ever
    // renames it the loop finds nothing and the bar stays native.
    unsafe {
        let view: &AnyObject = &*(h.ns_view.as_ptr() as *const AnyObject);
        let win: Option<&AnyObject> = msg_send![view, window];
        let Some(win) = win else { return };
        let content: Option<&AnyObject> = msg_send![win, contentView];
        let Some(content) = content else { return };
        let frame: Option<&AnyObject> = msg_send![content, superview];
        let Some(frame) = frame else { return };
        let subviews: &AnyObject = msg_send![frame, subviews];
        let n: usize = msg_send![subviews, count];
        for i in 0..n {
            let sv: &AnyObject = msg_send![subviews, objectAtIndex: i];
            if sv.class().name().to_bytes() != b"NSTitlebarContainerView" {
                continue;
            }
            let _: () = msg_send![sv, setWantsLayer: Bool::YES];
            let layer: Option<&AnyObject> = msg_send![sv, layer];
            let Some(layer) = layer else { continue };
            let cg: *const CGColor = match wear {
                Some(([r, g, b], _)) => {
                    let c: &AnyObject = msg_send![
                        class!(NSColor),
                        colorWithSRGBRed: f64::from(r) / 255.0,
                        green: f64::from(g) / 255.0,
                        blue: f64::from(b) / 255.0,
                        alpha: 1.0f64
                    ];
                    msg_send![c, CGColor]
                }
                None => std::ptr::null(),
            };
            let _: () = msg_send![layer, setBackgroundColor: cg];
            // nil hands the ink back to the window's (the OS's) appearance.
            let look: Option<&AnyObject> = match wear {
                Some((_, dark)) => {
                    let name = objc2_foundation::NSString::from_str(if dark {
                        "NSAppearanceNameDarkAqua"
                    } else {
                        "NSAppearanceNameAqua"
                    });
                    msg_send![class!(NSAppearance), appearanceNamed: &*name]
                }
                None => None,
            };
            let _: () = msg_send![sv, setAppearance: look];
        }
    }
}

/// Elsewhere the title bar is the window manager's, and a transparent client
/// area does not make it see-through.
#[cfg(not(target_os = "macos"))]
pub fn paint(_window: &Window, _wear: Wear) {}

/// Put the window's OWN glass in step with `opacity`: non-opaque below 1.0
/// (so crew's alpha reaches the desktop at all) and FROSTED — the desktop
/// behind a sheer pane or the nav is blurred by the window server
/// (`CGSSetWindowBackgroundBlurRadius` on macOS, via winit), so a sheer crew
/// reads as a pane of frosted glass rather than a hole in the window. Both
/// drop back off at full opacity, where the window is opaque again.
pub fn apply_window(window: &Window, opacity: f32) {
    let sheer = crate::config::wants_window_transparency(opacity);
    window.set_transparent(sheer);
    window.set_blur(sheer);
}

/// What the title bar should wear at `opacity`: the page colour and the
/// page's ink while the window is sheer, the OS's own bar (`None`) while it
/// is solid.
pub fn wanted(opacity: f32, page_bg: (u8, u8, u8), dark: bool) -> Wear {
    let (r, g, b) = page_bg;
    crate::config::wants_window_transparency(opacity).then_some(([r, g, b], dark))
}

#[cfg(test)]
mod tests {
    use super::wanted;

    #[test]
    fn a_solid_window_leaves_the_native_title_bar_alone() {
        assert_eq!(wanted(1.0, (1, 2, 3), false), None);
    }

    #[test]
    fn a_sheer_window_paints_its_title_bar_the_page_colour() {
        assert_eq!(wanted(0.88, (1, 2, 3), true), Some(([1, 2, 3], true)));
        assert_eq!(
            wanted(crate::config::MIN_WINDOW_OPACITY, (9, 9, 9), true),
            Some(([9, 9, 9], true))
        );
    }

    /// A light page gets a dark title whatever the OS is in: on a dark Mac
    /// the bar crew painted light carried the system's white title.
    #[test]
    fn the_painted_bar_carries_the_page_s_own_ink() {
        assert_eq!(
            wanted(0.8, (250, 250, 250), false),
            Some(([250, 250, 250], false))
        );
        assert_eq!(wanted(0.8, (10, 10, 10), true), Some(([10, 10, 10], true)));
    }
}
