//! The window server's blur behind a see-through window, at a radius of
//! crew's choosing. winit's `set_blur` is a fixed 80 pt — fog: what is behind
//! the window keeps its colours and loses its shapes. Liquid glass wants the
//! shapes (`LiquidStyle::desktop_blur`), so `titlebar::apply_window` follows
//! winit's call with this one.
use winit::window::Window;

/// Set the window server's blur behind the window to `radius` pt — what
/// winit's `set_blur` does, at a radius of our choosing.
#[cfg(target_os = "macos")]
pub fn radius(window: &Window, radius: i64) {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    // The private CoreGraphics pair winit's `set_blur` (and Terminal.app) use.
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGSMainConnectionID() -> *mut AnyObject;
        fn CGSSetWindowBackgroundBlurRadius(cid: *mut AnyObject, wid: isize, r: i64) -> i32;
    }
    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::AppKit(h) = handle.as_raw() else {
        return;
    };
    // SAFETY: winit hands out a live NSView for as long as `window` lives, we
    // are on the main thread, and `window` / `windowNumber` are public AppKit.
    unsafe {
        let view: &AnyObject = &*(h.ns_view.as_ptr() as *const AnyObject);
        let win: Option<&AnyObject> = msg_send![view, window];
        let Some(win) = win else { return };
        let n: isize = msg_send![win, windowNumber];
        CGSSetWindowBackgroundBlurRadius(CGSMainConnectionID(), n, radius);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn radius(_window: &Window, _radius: i64) {}
