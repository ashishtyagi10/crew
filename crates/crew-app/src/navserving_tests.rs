use super::*;
use crate::usageledger::{WindowStat, Windows};

fn text(cells: &[CellView], row: u16) -> String {
    let mut v: Vec<&CellView> = cells.iter().filter(|c| c.row == row).collect();
    v.sort_by_key(|c| c.col);
    v.iter().map(|c| c.c).collect::<String>().trim().to_string()
}

/// The three lines say who serves and how much of each window is spent; the
/// meters carry the same fractions in row order for the paint pass.
#[test]
fn who_and_windows_with_meter_fractions() {
    let s = Serving {
        provider: Some("claude-code".into()),
        model: Some("claude-sonnet-5".into()),
        windows: Windows {
            five_h: Some(WindowStat {
                left_ms: 2 * 3_600_000 + 10 * 60_000,
                spent: 25,
                budget: 100,
            }),
            seven_d: None,
        },
    };
    let (cells, meters) = serving_cells(&s, 40);
    assert!(text(&cells, 0).contains("SERVING"));
    assert_eq!(text(&cells, 1), "claude-code \u{00b7} claude-sonnet-5");
    assert!(
        text(&cells, 2).starts_with("5h \u{2593}\u{2593}"),
        "{}",
        text(&cells, 2)
    );
    assert!(text(&cells, 2).ends_with("2h10m"));
    assert!(
        text(&cells, 3).ends_with("\u{2014}"),
        "no 7d window yet: {}",
        text(&cells, 3)
    );
    assert_eq!(meters, vec![0.25, 0.0]);
    // The provider wears the accent, the model the ink.
    let p = cells.iter().find(|c| c.row == 1 && c.c == 'c').unwrap();
    let m = cells.iter().find(|c| c.row == 1 && c.c == '5').unwrap();
    assert_ne!(p.fg, m.fg);
}

/// No provider at all names the way in; a narrow card clips with `…`.
#[test]
fn no_provider_points_at_model_and_narrow_clips() {
    let (cells, _) = serving_cells(&Serving::default(), 40);
    assert_eq!(text(&cells, 1), "no provider \u{2014} /model");
    let s = Serving {
        provider: Some("openrouter".into()),
        model: Some("qwen/qwen3-coder-plus".into()),
        windows: Windows::default(),
    };
    let (cells, _) = serving_cells(&s, 20);
    let who = text(&cells, 1);
    assert!(who.ends_with('\u{2026}'), "{who}");
    assert!(cells.iter().all(|c| c.col < 20));
}

/// On a narrow nav the pair gives way to the model alone, and the meter
/// rows drop their countdown before clipping it to nothing.
#[test]
fn narrow_keeps_the_model_and_drops_the_countdown() {
    let s = Serving {
        provider: Some("claude-code".into()),
        model: Some("claude-sonnet-5".into()),
        windows: Windows {
            five_h: Some(WindowStat {
                left_ms: 3_600_000,
                spent: 50,
                budget: 100,
            }),
            seven_d: None,
        },
    };
    let (cells, _) = serving_cells(&s, 20);
    assert_eq!(text(&cells, 1), "claude-sonnet-5");
    assert_eq!(text(&cells, 2), format!("5h {}", bar(50)));
    let (cells, _) = serving_cells(&s, 40);
    assert_eq!(text(&cells, 1), "claude-code \u{00b7} claude-sonnet-5");
    assert!(text(&cells, 2).ends_with("1h00m"));
}
