//! On main the working badges wore a fixed roster colour: nothing on the
//! footer said which of three working agents was actually talking.
use super::*;
use crate::chatsummary::{footer_lines_with, FooterCtx};
use crate::motion::{set_level, MotionLevel};
use crate::summaryroute::FCell;
use crew_theme::contrast_ratio;

fn text(line: &[FCell]) -> String {
    line.iter().map(|c| c.0).collect()
}

/// The block under the `@coder` badge's label cells on line 3.
fn coder_block(map: &HashMap<String, u64>, now: u64) -> Color {
    let ctx = HashMap::new();
    let fc = FooterCtx {
        agents: &[],
        ctx: &ctx,
        tok_in: 0,
        tok_out: 0,
        cost_microusd: 0,
        branch: None,
        input: "",
        running_tasks: &[],
        plan_pending: false,
        active: vec!["coder"],
        cwd: None,
        windows: crate::usageledger::Windows::default(),
        readouts: Box::leak(Box::default()),
        pulse: Some(Pulses { map, now }),
    };
    let l3 = &footer_lines_with(&fc, 120, &mut Vec::new())[2];
    let s = text(l3);
    let at = s.find("@coder").unwrap_or_else(|| panic!("{s:?}"));
    let at = s[..at].chars().count(); // cells are per char, not per byte
    let cells = &l3[at..at + "@coder".len()];
    let block = cells[0].2.expect("a badge sits on a block");
    assert!(
        cells.iter().all(|c| c.2 == Some(block)),
        "one block per badge"
    );
    block
}

/// Block colour ≠ roster at +10 ms after a burst, == roster at +PULSE_MS,
/// and dimmer than roster (toward the page) once IDLE_MS have passed —
/// with the label's ink clearing the text floor on every one of them.
#[test]
fn a_badge_lights_with_its_tokens_settles_and_dims_when_idle() {
    let _g = crate::app::theme_test_guard();
    set_level(MotionLevel::Full);
    let _plain = crate::glyphs::force(false);
    let th = crew_theme::theme();
    let roster = crate::chatroster::agent_color("coder");
    let map: HashMap<String, u64> = [("coder".to_string(), 1_000u64)].into();

    assert_eq!(coder_block(&HashMap::new(), 1_010), roster, "never burst");
    let lit = coder_block(&map, 1_010);
    assert_ne!(lit, roster, "lit toward the ink right after the burst");
    assert_eq!(
        coder_block(&map, 1_000 + crate::shimmer::PULSE_MS),
        roster,
        "back on the roster colour after PULSE_MS"
    );
    let dim = coder_block(&map, 1_000 + IDLE_MS);
    assert_ne!(dim, roster, "idle: a dimmer block");
    assert!(
        contrast_ratio(dim, th.page_bg) < contrast_ratio(roster, th.page_bg),
        "dim sinks toward the page: {dim:?} vs {roster:?}"
    );
    assert!(contrast_ratio(dim, th.page_bg) >= crew_theme::readable::MARK_FLOOR);
    // The label stays readable on every block it is handed.
    let floor = crew_theme::contrast::text_floor();
    for block in [lit, roster, dim] {
        let ink = crate::segment::page_ink(block);
        assert!(contrast_ratio(ink, block) >= floor, "{ink:?} on {block:?}");
    }
}

#[test]
fn off_never_lights_a_badge() {
    let _g = crate::app::theme_test_guard();
    set_level(MotionLevel::Off);
    let _plain = crate::glyphs::force(false);
    let roster = crate::chatroster::agent_color("coder");
    let map: HashMap<String, u64> = [("coder".to_string(), 1_000u64)].into();
    assert_eq!(coder_block(&map, 1_010), roster);
    set_level(MotionLevel::Full);
}
