use super::*;

#[test]
fn sounds_monospace_catches_coding_fonts_only() {
    for name in ["JetBrains Mono", "Fira Code", "Consolas", "Menlo", "Monaco"] {
        assert!(sounds_monospace(name), "{name} should read as monospace");
    }
    for name in ["Helvetica", "Times New Roman", "Arial"] {
        assert!(!sounds_monospace(name), "{name} should not");
    }
}

#[test]
fn blocked_faces_never_appear() {
    assert!(is_blocked("Courier"));
    assert!(is_blocked("Courier New"));
    assert!(is_blocked("PT Mono"));
    assert!(is_blocked("Andale Mono"));
    assert!(is_blocked("Consolas"));
    assert!(!is_blocked("JetBrains Mono"));
    assert!(!is_blocked("MonoLisa"));
    // Even installed, a blocked face must not survive the family scan.
    let mut fs = crate::embedfont::font_system();
    let names = monospace_families(&mut fs);
    assert!(
        !names.iter().any(|n| is_blocked(n)),
        "a blocked face leaked into the monospace list: {names:?}"
    );
}

#[test]
fn monospace_families_sorted_and_deduped() {
    let mut fs = crate::embedfont::font_system();
    let names = monospace_families(&mut fs);
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "names must be sorted");
    let mut deduped = names.clone();
    deduped.dedup();
    assert_eq!(deduped.len(), names.len(), "names must be de-duplicated");
}

#[test]
fn proportional_and_symbol_noise_is_excluded() {
    // These ship flagged monospaced on macOS but are not coding faces:
    // Arial Unicode MS is proportional; Symbols Nerd Font Mono has no
    // Latin letters. If installed, the measured policy must drop them.
    let mut fs = crate::embedfont::font_system();
    let names = monospace_families(&mut fs);
    for noise in ["Arial Unicode MS", "Symbols Nerd Font Mono"] {
        assert!(
            !names.iter().any(|n| n == noise),
            "{noise} should fail the fixed-pitch-Latin check"
        );
    }
}
