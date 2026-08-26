//! Does the OS ask for less motion? — the other half of `motion = auto`.
//!
//! macOS exposes an accessibility switch (Settings → Accessibility → Display →
//! "Reduce motion") that every well-behaved app is expected to honor: it is the
//! desktop equivalent of the web's `prefers-reduced-motion`, and it exists
//! because vestibular disorders make sliding, parallax and zoom actively
//! unpleasant rather than merely decorative. crew already has a genuine `off`
//! (durations collapse to zero, see [`crate::motion`]) — this is what lets the
//! user get it without knowing crew has a setting at all.
//!
//! Read where the answer can change (startup, config adoption, the appearance
//! notification), never per frame: [`crate::motion::set_os_reduce`] caches it
//! in an atomic that the render path reads instead.
//!
//! Everywhere crew cannot ask this question, the answer is `false` — no OS
//! request on record, so `auto` behaves as full motion.

/// Whether the OS is asking apps to reduce motion.
#[cfg(target_os = "macos")]
pub(crate) fn reduce_motion() -> bool {
    objc2_app_kit::NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion()
}

/// Non-macOS: no portable way to ask, so `auto` means full motion. (A future
/// Windows port reads `SystemParametersInfo(SPI_GETCLIENTAREAANIMATION)`;
/// GNOME/KDE both expose `gtk-enable-animations` / the XDG appearance portal.)
#[cfg(not(target_os = "macos"))]
pub(crate) fn reduce_motion() -> bool {
    false
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    /// The binding reads the same preference the `defaults` CLI does, so the
    /// two must agree on whatever this machine happens to be set to. Written
    /// because the failure mode is silent: a wrong selector or a hard-wired
    /// `false` looks exactly like "the user never asked for reduced motion",
    /// which is the common case and would pass any weaker test.
    ///
    /// `com.apple.universalaccess reduceMotion` is what the Accessibility pane
    /// writes; a missing key exits non-zero and means off.
    #[test]
    fn the_probe_agrees_with_the_defaults_cli() {
        let out = std::process::Command::new("defaults")
            .args(["read", "com.apple.universalaccess", "reduceMotion"])
            .output()
            .expect("`defaults` exists on every macOS");
        let cli = out.status.success() && String::from_utf8_lossy(&out.stdout).trim() == "1";
        assert_eq!(
            super::reduce_motion(),
            cli,
            "probe disagreed with `defaults read com.apple.universalaccess reduceMotion`"
        );
    }
}
