use super::*;
use crate::chattool::ToolDone;
use crate::md::syntax::Token;
use crate::motion::set_level;
use crate::motion::MotionLevel::{Full, Off};

fn line(label: &str, args: &str, started: u64) -> ToolLine {
    ToolLine {
        label: label.into(),
        args_short: args.into(),
        started_ms: started,
        done: None,
        show_text: false,
    }
}

fn done(ok: bool, ms: u64, text: &str) -> ToolLine {
    let mut l = line("fs:read", "src/foo.rs", 0);
    l.done = Some(ToolDone {
        ok,
        ms,
        text: text.into(),
    });
    l
}

fn text(l: &CardLine) -> String {
    l.iter().map(|c| c.c).collect()
}

fn is_pua(c: char) -> bool {
    matches!(u32::from(c), 0xE000..=0xF8FF | 0xF0000..=0xFFFFD | 0x100000..=0x10FFFD)
}

#[test]
fn durations_read_as_ms_then_seconds_then_minutes() {
    assert_eq!(fmt_ms(120), "120 ms");
    assert_eq!(fmt_ms(3_200), "3.2 s");
    assert_eq!(fmt_ms(64_000), "1m 04s");
    assert_eq!(fmt_ms(0), "0 ms");
}

#[test]
fn a_pending_line_spins_and_counts_whole_seconds_up() {
    let _g = crate::app::motion_test_guard();
    set_level(Full);
    let l = line("fs:read", "src/foo.rs", 1_000);
    let at0 = text(&render(&l, 1_000, 60, false));
    assert_eq!(at0, "  | fs:read src/foo.rs", "no clock under a second");
    assert_eq!(
        text(&render(&l, 1_120, 60, false)),
        "  / fs:read src/foo.rs",
        "the next spinner frame"
    );
    assert_eq!(
        text(&render(&l, 4_300, 60, false)),
        "  \\ fs:read src/foo.rs \u{00b7} 3s"
    );
    let nerd = render(&l, 1_000, 60, true);
    assert!(is_pua(nerd[2].c), "a pie-slice frame on the icon set");
    assert_eq!(nerd[2].fg, crate::palette::accent());
}

#[test]
fn motion_off_pins_the_spinner_and_the_clock_moves_once_a_second() {
    let _g = crate::app::motion_test_guard();
    set_level(Off);
    let l = line("sys:run", "ls", 1_000);
    let a = text(&render(&l, 4_000, 60, true));
    assert_eq!(
        a, "  \u{2026} sys:run ls \u{00b7} 3s",
        "static … on any font"
    );
    assert_eq!(
        text(&render(&l, 4_500, 60, true)),
        a,
        "unchanged within the second"
    );
    assert_ne!(text(&render(&l, 5_000, 60, true)), a, "ticks at the second");
    set_level(Full);
}

#[test]
fn done_lines_carry_the_mark_in_the_added_or_removed_ink() {
    let _g = crate::app::motion_test_guard();
    let ok = render(&done(true, 120, ""), 0, 60, false);
    assert_eq!(text(&ok), "  \u{2713} fs:read src/foo.rs \u{00b7} 120 ms");
    assert_eq!(ok[2].fg, crate::chatink::token_fg(Token::Added));
    let bad = render(&done(false, 3_200, ""), 0, 60, false);
    assert_eq!(text(&bad), "  \u{2717} fs:read src/foo.rs \u{00b7} 3.2 s");
    assert_eq!(bad[2].fg, crate::chatink::token_fg(Token::Removed));
    let nerd = render(&done(true, 1, ""), 0, 60, true);
    assert!(is_pua(nerd[2].c) && nerd[2].c != ok[2].c);
}

#[test]
fn the_subject_is_clipped_so_the_timing_always_fits() {
    let _g = crate::app::motion_test_guard();
    let l = done(true, 120, "");
    let s = text(&render(&l, 0, 20, false));
    assert_eq!(s.chars().count(), 20, "{s:?}");
    assert!(s.ends_with(" \u{00b7} 120 ms"), "{s:?}");
    assert!(s.contains('\u{2026}'), "the subject took the cut: {s:?}");
}

#[test]
fn a_result_with_text_previews_its_first_line_muted_after_the_timing() {
    let _g = crate::app::theme_test_guard(); // the same lock; this reads the theme
    let l = done(true, 120, "\n  fn main() {}\nmore");
    let r = render(&l, 0, 60, false);
    let s = text(&r);
    assert_eq!(
        s,
        "  \u{2713} fs:read src/foo.rs \u{00b7} 120 ms  fn main() {}"
    );
    assert_eq!(r.last().unwrap().fg, crew_theme::theme().text_muted);
    assert!(text_rows(&l, 60).is_empty(), "closed until clicked");
}

#[test]
fn opened_text_fills_a_code_field_capped_at_twelve_rows() {
    let _g = crate::app::motion_test_guard();
    let body: Vec<String> = (1..=20).map(|i| format!("line {i}")).collect();
    let mut l = done(true, 1, &body.join("\n"));
    l.show_text = true;
    let rows = text_rows(&l, 30);
    assert_eq!(rows.len(), TEXT_ROWS);
    assert_eq!(text(&rows[0]), format!("{:<30}", "    line 1"));
    assert_eq!(text(&rows[11]).trim_end(), "    \u{2026} +9 lines");
    let bg = crate::chatink::code_bg();
    assert!(rows
        .iter()
        .flatten()
        .all(|c| c.bg == Some(bg) && c.c != '\t'));
    assert!(rows.iter().all(|r| r.len() == 30), "a full-width field");
    l.done.as_mut().unwrap().text = "one".into();
    assert_eq!(text_rows(&l, 30).len(), 1);
}
