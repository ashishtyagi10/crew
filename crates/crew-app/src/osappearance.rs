//! Does the OS switch its own appearance? — the other half of `auto`.
//!
//! macOS has three appearance settings, and only two of them are visible
//! through winit: Light and Dark both arrive as a `ThemeChanged`/`theme()`
//! value, and so does Auto — which is a SCHEDULE, not a state. crew needs the
//! distinction because `auto` means something different in each case: while
//! macOS is on Auto it already encodes the user's day/night intent and crew
//! should just follow it, but a PINNED appearance never changes, so following
//! it makes `auto` a permanent synonym for that one side.
//!
//! Everywhere crew cannot ask this question, the answer is `true` — the OS is
//! taken as authoritative and `auto` behaves exactly as it always has.

/// Whether the OS switches its own appearance between light and dark.
#[cfg(target_os = "macos")]
pub(crate) fn switches_automatically() -> bool {
    use objc2_foundation::{NSString, NSUserDefaults};
    // `AppleInterfaceStyleSwitchesAutomatically` is set only by Appearance:
    // Auto. Light and Dark both leave it absent, which reads as false — the
    // pinned case, which is the one that matters.
    let key = NSString::from_str("AppleInterfaceStyleSwitchesAutomatically");
    NSUserDefaults::standardUserDefaults().boolForKey(&key)
}

/// Non-macOS: no way to tell a pinned appearance from a scheduled one, so
/// treat whatever the OS reports as the whole truth (`auto` keeps following
/// it, unchanged).
#[cfg(not(target_os = "macos"))]
pub(crate) fn switches_automatically() -> bool {
    true
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    /// The binding reads the same preference the `defaults` CLI does, so the
    /// two must agree on whatever this machine happens to be set to. Written
    /// because the failure mode is silent: a wrong key or a wrong domain just
    /// reads `false`, which looks exactly like a pinned appearance — and a
    /// permanently-`false` probe would put every Mac on the clock fallback.
    ///
    /// Note what this can and cannot catch: on a Mac with Appearance pinned
    /// (the common case, and the one this whole fallback exists for) both
    /// sides read false, so a wrong KEY still passes here. That half is
    /// covered by `bool_for_key_actually_reads_a_value` below; only a Mac set
    /// to Auto exercises the true branch end to end.
    #[test]
    fn the_probe_agrees_with_the_defaults_cli() {
        let out = std::process::Command::new("defaults")
            .args(["read", "-g", "AppleInterfaceStyleSwitchesAutomatically"])
            .output()
            .expect("`defaults` exists on every macOS");
        // A missing key exits non-zero — Appearance is Light or Dark, pinned.
        let cli = out.status.success() && String::from_utf8_lossy(&out.stdout).trim() == "1";
        assert_eq!(
            super::switches_automatically(),
            cli,
            "probe disagreed with `defaults read -g AppleInterfaceStyleSwitchesAutomatically`"
        );
    }

    /// `boolForKey` is not hard-wired to false. Without this, a probe that
    /// could never return true would pass the comparison above on every
    /// pinned Mac — which is most of them.
    #[test]
    fn bool_for_key_actually_reads_a_value() {
        use objc2_foundation::{NSString, NSUserDefaults};
        // A test-only key in this binary's own domain: set it, read it back
        // through the exact call `switches_automatically` uses, remove it.
        let key = NSString::from_str("crew.test.osappearance.probe");
        let defaults = NSUserDefaults::standardUserDefaults();
        defaults.setBool_forKey(true, &key);
        assert!(defaults.boolForKey(&key), "true must read back as true");
        defaults.setBool_forKey(false, &key);
        assert!(!defaults.boolForKey(&key), "false must read back as false");
        defaults.removeObjectForKey(&key);
    }
}
