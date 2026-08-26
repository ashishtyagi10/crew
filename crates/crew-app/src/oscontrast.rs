//! Does the OS want more contrast? — the probe behind `contrast = auto`.
//!
//! Sibling of [`crate::reducemotion`] and [`crate::osappearance`], read at the
//! same three points those are (startup, config adoption, the appearance
//! notification) and cached the same way: `crew_theme::contrast` holds the
//! answer in an atomic, because the floors it moves are consulted while
//! building frames.
//!
//! Everywhere crew cannot ask, the answer is `false` — no request on record,
//! so `auto` is the ordinary WCAG AA floors. The user can still say so
//! themselves with `/contrast high`.

/// Whether the OS is asking apps to increase contrast.
#[cfg(target_os = "macos")]
pub(crate) fn increase_contrast() -> bool {
    objc2_app_kit::NSWorkspace::sharedWorkspace().accessibilityDisplayShouldIncreaseContrast()
}

/// Non-macOS: no portable probe, so `auto` means the AA floors. (Windows
/// exposes it as the High Contrast theme via `SystemParametersInfo`;
/// GNOME/KDE ship a `HighContrast` icon/GTK theme rather than a flag.)
#[cfg(not(target_os = "macos"))]
pub(crate) fn increase_contrast() -> bool {
    false
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    /// The binding reads the same preference the `defaults` CLI does. Written
    /// for the same reason its two siblings were: the failure mode is silent —
    /// a wrong selector reads `false`, which looks exactly like "the user
    /// never asked", the common case that any weaker test would pass on.
    #[test]
    fn the_probe_agrees_with_the_defaults_cli() {
        let out = std::process::Command::new("defaults")
            .args(["read", "com.apple.universalaccess", "increaseContrast"])
            .output()
            .expect("`defaults` exists on every macOS");
        let cli = out.status.success() && String::from_utf8_lossy(&out.stdout).trim() == "1";
        assert_eq!(
            super::increase_contrast(),
            cli,
            "probe disagreed with `defaults read com.apple.universalaccess increaseContrast`"
        );
    }
}
