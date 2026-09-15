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

#[test]
fn one_typeface_takes_one_row_however_many_names_it_is_installed_under() {
    let names = one_per_typeface(
        [
            "JetBrainsMono NF",
            "JetBrainsMono NFM",
            "JetBrainsMono Nerd Font",
            "JetBrainsMono Nerd Font Mono",
            "FiraCode Nerd Font",
            "FiraCode Nerd Font Mono",
            "MonoLisa",
        ]
        .map(String::from)
        .to_vec(),
    );
    assert_eq!(
        names,
        vec![
            "FiraCode Nerd Font Mono".to_string(),
            "JetBrainsMono Nerd Font Mono".to_string(),
            "MonoLisa".to_string(),
        ],
        "four JetBrains rows and two FiraCode rows are three faces"
    );
}

#[test]
fn the_row_that_survives_is_the_icon_bearing_one() {
    // A Nerd Font Mono build is the same outlines plus one-cell marks, so it
    // is the name worth offering when the machine has both.
    for (names, want) in [
        (vec!["Lilex", "Lilex Nerd Font"], "Lilex Nerd Font"),
        (
            vec!["Comic Mono", "ComicMono Nerd Font Mono"],
            "ComicMono Nerd Font Mono",
        ),
        (vec!["IBM Plex Mono"], "IBM Plex Mono"),
    ] {
        let got = one_per_typeface(names.iter().map(|s| s.to_string()).collect());
        assert_eq!(got, vec![want.to_string()], "from {names:?}");
    }
}

#[test]
fn two_unrelated_faces_are_never_folded_into_one_row() {
    let names = [
        "Cascadia Code",
        "Cascadia Mono",
        "Geist Mono",
        "Google Sans Code",
        "IBM Plex Mono",
        "Menlo",
        "MonoLisa",
        "Operator Mono",
        "SF Mono",
    ]
    .map(String::from)
    .to_vec();
    assert_eq!(
        one_per_typeface(names.clone()).len(),
        names.len(),
        "a distinct face was swallowed"
    );
}

#[test]
fn the_os_private_faces_are_not_offered() {
    // macOS hides `.SF NS Mono` from every font menu it draws: it is the
    // system's own copy, and the user already has SF Mono by name.
    assert!(is_private(".SF NS Mono"));
    assert!(!is_private("SF Mono"));
    let mut fs = crate::embedfont::font_system();
    let names = monospace_families(&mut fs);
    assert!(
        !names.iter().any(|n| n.starts_with('.')),
        "a private system face was listed: {names:?}"
    );
}
