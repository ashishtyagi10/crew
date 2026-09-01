use super::*;
use crate::chatmsgs::tests::msg;
use crate::chatmsgs::{card_lines, View};

#[test]
fn splash_renders_headerless_and_centered() {
    // The startup nameplate: no `agent smith · time` header line above it,
    // and every line centered in the pane width.
    let art = "\u{2554}\u{2550}\u{2550}\u{2557}\n\u{2551} AGENT \u{2551}\n\u{255a}\u{2550}\u{2550}\u{255d}";
    let m = msg("agent smith", art);
    assert!(
        is_splash(&m),
        "nameplate art must be detected as the splash"
    );
    let lines = card_lines(&[&m], 40, 0, View::default());
    let texts: Vec<String> = lines
        .iter()
        .map(|l| l.iter().map(|c| c.c).collect())
        .collect();
    assert!(
        !texts.iter().any(|t| t.contains("agent smith")),
        "no header line on the splash: {texts:?}"
    );
    let top = texts
        .iter()
        .find(|t| t.contains('\u{2554}'))
        .expect("box top present");
    let lead = top.chars().take_while(|c| *c == ' ').count();
    assert!(lead > 10, "box must be centered, got lead {lead}: {top:?}");
}

#[test]
fn fade_t_ramps_with_message_age() {
    // Counting pass (now == 0) and unstamped messages render fully drawn.
    assert_eq!(fade_t("1000", 0), 1.0);
    assert_eq!(fade_t("", 5_000), 1.0);
    // A just-landed message starts faded and finishes after FADE_MS.
    assert_eq!(fade_t("5000", 5_000), 0.0);
    let mid = fade_t("5000", 5_000 + FADE_MS / 2);
    assert!(mid > 0.4 && mid < 0.6, "got: {mid}");
    assert_eq!(fade_t("5000", 5_000 + FADE_MS), 1.0);
}
