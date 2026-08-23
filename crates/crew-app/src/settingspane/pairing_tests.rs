use super::*;
use crate::config::CrewConfig;

/// Every value the config accepts is reachable, and every value the picker
/// produces parses back to the same thing. This is the property that keeps a
/// Save from dropping a setting the form could display but not produce.
#[test]
fn the_list_round_trips_every_value_the_config_accepts() {
    for v in values() {
        let Some(sel) = v else {
            continue;
        };
        let stored = sel.label();
        assert_eq!(
            crew_theme::parse_selection(stored),
            Some(sel),
            "`{stored}` does not parse back to what the picker meant"
        );
        // ...and through the config, which is the path a Save actually takes.
        let cfg = CrewConfig::from_toml_str(&format!("theme_dark = \"{stored}\"\n"));
        assert_eq!(
            cfg.auto_pool_selections().0,
            Some(sel),
            "`{stored}` was lost on the way through the config"
        );
    }
}

/// Every palette and every pool is offered — the picker is not a subset of
/// what the config field means.
#[test]
fn every_pool_and_every_palette_is_reachable() {
    let all = values();
    for m in crew_theme::THEME_MODES {
        // `auto` is the one mode a side may not hold: it would be its own
        // pairing. `auto_pool_selections` drops it, so offering it would be a
        // picker entry that does nothing.
        let want = Some(crew_theme::Selection::Mode(m));
        let offered = all.contains(&want);
        assert_eq!(
            offered,
            m != crew_theme::RandomMode::Auto,
            "{} is {}offered",
            m.as_str(),
            if offered { "" } else { "not " }
        );
    }
    for id in crew_theme::ALL_THEMES {
        assert!(
            all.contains(&Some(crew_theme::Selection::Fixed(id))),
            "{} is not reachable in the picker",
            id.as_str()
        );
    }
    assert_eq!(all.first(), Some(&None), "unset must lead the list");
    assert_eq!(all.len(), 1 + 3 + crew_theme::ALL_THEMES.len());
}

/// Cycling walks the whole list and wraps, forwards and backwards.
#[test]
fn cycling_visits_every_entry_and_wraps() {
    let all = values();
    let mut seen = Vec::new();
    let mut cur: Option<crew_theme::Selection> = None;
    for _ in 0..all.len() {
        seen.push(cur);
        cur = cycle(cur, false).and_then(|s| crew_theme::parse_selection(&s));
    }
    assert_eq!(seen, all, "forward cycle did not visit the list in order");
    assert_eq!(cur, None, "forward cycle did not wrap to the start");

    // Backwards from unset lands on the last entry, which is what makes the
    // far end of a thirteen-entry list one keypress away.
    let back = cycle(None, true).and_then(|s| crew_theme::parse_selection(&s));
    assert_eq!(back, *all.last().unwrap());
}

/// A config string this build does not recognise enters at `default` instead
/// of wedging the picker on an index it can never match.
#[test]
fn an_unrecognised_side_enters_the_cycle_at_default() {
    let cfg = CrewConfig::from_toml_str("theme_dark = \"harvest-gold\"\n");
    let (dark, _) = cfg.auto_pool_selections();
    assert_eq!(dark, None, "an unknown palette must parse to no pairing");
    assert_eq!(label(dark), DEFAULT_LABEL);
    assert_eq!(cycle(dark, false).as_deref(), Some("dark"));

    // The fallback itself, which the config path above cannot reach (anything
    // unparseable arrives as `None`, and `None` IS the first list entry). The
    // one value that is genuinely absent is `auto` — a side may not be its own
    // pairing — so that is what proves the cycle does not wedge on it.
    let orphan = Some(crew_theme::Selection::Mode(crew_theme::RandomMode::Auto));
    assert!(!values().contains(&orphan), "auto must not be offered");
    assert_eq!(cycle(orphan, false).as_deref(), Some("dark"));
    assert_eq!(
        cycle(orphan, true).as_deref(),
        values().last().unwrap().map(|s| s.label()),
        "stepping back off an orphan must reach the list, not panic"
    );
}

/// `auto` as a side would be its own pairing; the config drops it and the
/// picker must agree rather than showing a value that does nothing.
#[test]
fn auto_as_a_side_shows_as_default() {
    let cfg = CrewConfig::from_toml_str("theme_light = \"auto\"\n");
    assert_eq!(label(cfg.auto_pool_selections().1), DEFAULT_LABEL);
}
