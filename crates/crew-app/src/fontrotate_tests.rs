use super::*;

fn pool() -> Vec<String> {
    vec!["Menlo".into(), "Monaco".into(), "Hack".into()]
}

#[test]
fn pick_never_returns_the_current_family() {
    for seed in 0..50u64 {
        let p = pick(&pool(), Some("Menlo"), seed).unwrap();
        assert_ne!(p, "Menlo", "seed {seed}");
    }
}

#[test]
fn pick_is_deterministic_for_a_seed() {
    assert_eq!(
        pick(&pool(), Some("Menlo"), 7),
        pick(&pool(), Some("Menlo"), 7)
    );
}

#[test]
fn pick_returns_none_when_no_alternative_exists() {
    assert_eq!(pick(&["Menlo".to_string()], Some("Menlo"), 1), None);
    assert_eq!(pick(&[], None, 1), None);
}

#[test]
fn due_gates_on_the_shared_rotate_clock() {
    let mut r = FontRotate {
        on: true,
        last_ms: 1_000,
        ..Default::default()
    };
    assert!(!r.due(1_000 + crew_theme::ROTATE_MS - 1));
    assert!(r.due(1_000 + crew_theme::ROTATE_MS));
    r.on = false;
    assert!(!r.due(1_000 + crew_theme::ROTATE_MS));
}

/// How often each family is picked across a long run of seeds — the only way
/// to read a weighting, which is a statement about a distribution and not
/// about any one draw.
fn tally(pool: &[String], current: Option<&str>) -> std::collections::HashMap<String, usize> {
    let mut out = std::collections::HashMap::new();
    for seed in 0..3_000u64 {
        if let Some(f) = pick(pool, current, seed) {
            *out.entry(f).or_insert(0) += 1;
        }
    }
    out
}

fn owned(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn a_favourite_comes_up_about_three_times_as_often_as_anything_else() {
    let pool = owned(&["MonoLisa", "Menlo", "Geist Mono", "Google Sans Code"]);
    let t = tally(&pool, Some("Lilex"));
    let fav = t["MonoLisa"];
    for plain in ["Menlo", "Geist Mono", "Google Sans Code"] {
        let n = t[plain];
        let ratio = fav as f32 / n as f32;
        assert!(
            (2.4..3.6).contains(&ratio),
            "MonoLisa came up {fav}× and {plain} {n}× — ratio {ratio:.2}, not ~{}",
            crew_theme::FAVORITE_WEIGHT
        );
    }
}

#[test]
fn the_rotation_still_reaches_every_face_in_the_pool() {
    // Weighted, not exclusive: a favourite list that silently retired the rest
    // of the pool would be a different feature.
    let pool = owned(&["MonoLisa", "Menlo", "Geist Mono", "Noto Sans Mono"]);
    let t = tally(&pool, None);
    for fam in ["Menlo", "Geist Mono", "Noto Sans Mono"] {
        assert!(t.get(fam).copied().unwrap_or(0) > 0, "{fam} never came up");
    }
}

#[test]
fn the_face_on_screen_is_out_of_the_draw_in_every_spelling() {
    // Rotating from `JetBrainsMono Nerd Font` to `JetBrains Mono` announces a
    // new font and changes nothing on the page.
    let pool = owned(&[
        "JetBrains Mono",
        "JetBrainsMono NF",
        "JetBrainsMono Nerd Font Mono",
        "Menlo",
    ]);
    for seed in 0..200u64 {
        let p = pick(&pool, Some("JetBrains Mono"), seed).unwrap();
        assert_eq!(
            p, "Menlo",
            "seed {seed} rotated to another spelling of the same face"
        );
    }
}

#[test]
fn one_typeface_gets_one_ticket_however_many_spellings_are_installed() {
    // Three JetBrains names and one Menlo: JetBrains is a favourite, so it
    // should come up ~3× Menlo — NOT 9×, which is what weighting names rather
    // than faces would give.
    let pool = owned(&[
        "JetBrains Mono",
        "JetBrainsMono NF",
        "JetBrainsMono Nerd Font Mono",
        "Menlo",
    ]);
    let t = tally(&pool, Some("Lilex"));
    let jet: usize = t
        .iter()
        .filter(|(k, _)| crew_theme::typeface_key(k) == "jetbrains")
        .map(|(_, n)| *n)
        .sum();
    let ratio = jet as f32 / t["Menlo"] as f32;
    assert!(
        (2.4..3.6).contains(&ratio),
        "JetBrains came up {jet}× to Menlo's {} — ratio {ratio:.2}",
        t["Menlo"]
    );
}

#[test]
fn the_icon_bearing_spelling_is_the_one_asked_for() {
    // Same face, three names: the Nerd Font *Mono* build carries the marks
    // crew draws with, at one cell each.
    let pool = owned(&[
        "JetBrains Mono",
        "JetBrainsMono Nerd Font",
        "JetBrainsMono Nerd Font Mono",
    ]);
    for seed in 0..100u64 {
        assert_eq!(
            pick(&pool, Some("Menlo"), seed).unwrap(),
            "JetBrainsMono Nerd Font Mono",
            "seed {seed}"
        );
    }
    // With no Nerd Font build installed, the plain name is what there is.
    let plain = owned(&["JetBrains Mono"]);
    assert_eq!(pick(&plain, Some("Menlo"), 1).unwrap(), "JetBrains Mono");
}
