use super::*;

#[test]
fn a_live_sub_cent_rate_never_badges_as_free() {
    // 5_000 µ$/Mtok = $0.000005/Mtok — nonzero, but rounds to "0.00" at
    // two decimals. Must render as a paid row, not "$0/$0" (free-looking).
    assert_eq!(dollars(5_000), "<0.01");
    let badge = price_badge(Some((5_000, 5_000)), false);
    assert_eq!(badge, "$<0.01/$<0.01");
    assert_ne!(badge, "$0/$0");
}

#[test]
fn ordinary_rates_are_unaffected() {
    assert_eq!(dollars(0), "0");
    assert_eq!(dollars(3_000_000), "3");
    assert_eq!(dollars(400_000), "0.4");
    assert_eq!(dollars(1_650_000), "1.65");
    assert_eq!(dollars(5_000_000), "5");
}

fn row_desc(slug: &str, today: i64) -> String {
    let m = crew_hive::catalog::catalog()
        .iter()
        .find(|m| m.slug == slug)
        .expect("a catalog row");
    desc(m, (m.price, m.free, m.context), Route::Mock, true, today)
}

#[test]
fn a_dated_row_says_when_right_after_its_slug() {
    let nov24 = crew_hive::catalog::sunset::days_from_civil(2026, 11, 24);
    let d = row_desc("gpt-5", nov24);
    assert!(
        d.starts_with("gpt-5 \u{b7} retires Dec 11 \u{b7} $1.25/$10"),
        "{d}"
    );
    let d = row_desc("gemini-2.5-pro", nov24);
    assert!(
        d.starts_with("gemini-2.5-pro \u{b7} replaced Dec 1 \u{b7} "),
        "{d}"
    );
    // Past the day the row still shows, marked, so a stale pin explains itself.
    let d = row_desc("gpt-4o", nov24 + 60);
    assert!(d.starts_with("gpt-4o \u{b7} retired \u{b7} "), "{d}");
}

#[test]
fn an_undated_row_reads_as_before() {
    let nov24 = crew_hive::catalog::sunset::days_from_civil(2026, 11, 24);
    let d = row_desc("gpt-5.5", nov24);
    assert_eq!(
        d,
        "gpt-5.5 \u{b7} $5/$30 \u{b7} 1M \u{b7} mock \u{b7} \u{25cf} current"
    );
    assert!(!row_desc("gpt-6-luna", nov24).contains("retire"));
}

/// The value picker for `/model gpt` drawn in a window, to look at the
/// badge beside its neighbours: `CREW_SHOT_DIR=<dir> cargo test -p crew-app
/// --bin crew model_sunset_shot -- --ignored`.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn model_sunset_shot() {
    let _g = crate::app::theme_test_guard();
    for (q, name) in [
        ("/model gpt", "model-sunset-gpt"),
        ("/model gemini", "model-sunset-gemini"),
    ] {
        let prep = |app: &mut crate::app::CrewApp| {
            app.input.focused = true;
            app.input.text = q.into();
        };
        let id = crew_theme::ThemeId::TokyoNight;
        if crate::crtglassshot_tests::window_with(name, id, 1.0, &[], &prep).is_none() {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        }
    }
}
