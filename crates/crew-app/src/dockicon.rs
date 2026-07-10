//! Sets the Dock icon at runtime. A symlink-executable bundle can lose
//! bundle identity and a plain terminal launch never had one — either way
//! the Dock would show the generic binary icon without this.
#![cfg(target_os = "macos")]

use objc2::{AnyThread, MainThreadMarker};
use objc2_app_kit::{NSApplication, NSImage};
use objc2_foundation::NSData;

/// Call on the main thread after the event loop (NSApplication) exists.
pub fn set() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let data = NSData::with_bytes(crate::appregister::ICON_PNG_512);
    let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    unsafe { app.setApplicationIconImage(Some(&image)) };
}
