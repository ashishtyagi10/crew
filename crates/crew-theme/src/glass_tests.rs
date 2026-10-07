use super::*;
use crate::{ALL_THEMES, CRT_GREEN, PAPER_DARK, PAPER_LIGHT};

#[test]
fn level_round_trips_and_accepts_aliases() {
    for l in [
        GlassLevel::Off,
        GlassLevel::Low,
        GlassLevel::Medium,
        GlassLevel::High,
    ] {
        assert_eq!(GlassLevel::parse(l.as_str()), Some(l));
    }
    assert_eq!(GlassLevel::parse("on"), Some(GlassLevel::Medium));
    assert_eq!(GlassLevel::parse("NONE"), Some(GlassLevel::Off));
    assert_eq!(GlassLevel::parse(" High "), Some(GlassLevel::High));
    assert_eq!(GlassLevel::parse("shiny"), None);
}

/// Liquid glass (2026-09-23): every page wears a sheet, and the sheet casts
/// a shadow — the depth is the point of it. Tubes included since 2026-10-05.
#[test]
fn every_page_wears_liquid_glass() {
    for id in ALL_THEMES {
        let s = style_for(id.theme()).scaled(GlassLevel::Medium);
        assert!(s.visible(), "{}: no sheet", id.as_str());
        assert!(s.shadow_alpha > 0.0, "{}: no shadow", id.as_str());
        assert!(s.highlight_alpha > 0.0, "{}: no rim", id.as_str());
    }
}

/// A tube is a terminal running in glass (2026-10-05) over a tinted
/// faceplate (crew-app's `tubesheer`): each pane is a sheet of SMOKED glass —
/// the page run a little toward its phosphor, so it is tinted and not black,
/// but dark, because milk under the text is what fogged it over a bright
/// desktop (2026-10-06, a day of milky frost: "this crt theme is ugly") —
/// grained, glossy across its face, rimmed in its phosphor run toward white,
/// and haloed: its shadow is the phosphor's light, not black.
#[test]
fn tubes_are_sheets_of_smoked_glass() {
    let tubes: Vec<_> = ALL_THEMES
        .into_iter()
        .filter(|id| id.theme().is_tube())
        .collect();
    assert!(!tubes.is_empty(), "the filter found no tubes");
    let paper = style_for(&PAPER_DARK);
    let top = |c: (u8, u8, u8)| {
        [c.0, c.1, c.2]
            .iter()
            .enumerate()
            .max_by_key(|x| x.1)
            .unwrap()
            .0
    };
    for id in tubes {
        let t = id.theme();
        let s = style_for(t);
        let (name, p) = (id.as_str(), t.border_focused);
        let (f, page) = (s.tint, t.page_bg);
        assert_eq!(top(f), top(p), "{name}: smoke {f:?} in its own phosphor");
        let max = |c: (u8, u8, u8)| c.0.max(c.1).max(c.2);
        assert!(
            max(f) > max(page) + 8 && max(f) <= 64,
            "{name}: smoke {f:?} is tinted, and dark"
        );
        assert!(
            s.alpha_top > paper.alpha_top && s.alpha_top <= 0.4 && s.alpha_bottom < s.alpha_top,
            "{name}: a sheet ({} .. {})",
            s.alpha_top,
            s.alpha_bottom
        );
        assert!(s.noise > 0.0, "{name}: grained");
        assert!(s.gloss > 0.0, "{name}: glossy");
        assert_eq!(s.glow, 1.0, "{name}: its shadow is a halo of light");
        let h = s.highlight;
        assert!(h.0 >= p.0 && h.1 >= p.1 && h.2 >= p.2, "{name}: rim {h:?}");
        assert_eq!(top(h), top(p), "{name}: rim {h:?} left the phosphor {p:?}");
    }
}

/// Frost stays frost: paper and modern sheets carry no gloss, and their
/// shadow is a shadow.
#[test]
fn frost_pages_have_no_gloss_and_a_black_shadow() {
    // Liquid glass is a slab, not frost: it carries a gloss (see below).
    let frost = |id: &crate::ThemeId| !id.theme().is_tube() && id.theme().liquid.is_none();
    for id in ALL_THEMES.into_iter().filter(frost) {
        let s = style_for(id.theme());
        assert_eq!((s.gloss, s.glow), (0.0, 0.0), "{}", id.as_str());
    }
}

/// A bright rim on a dark page reads as a neon outline, and a faint shadow on
/// a near-black page reads as nothing: the dark sheet trades one for the
/// other.
#[test]
fn dark_pages_soften_the_rim_and_deepen_the_shadow() {
    let light = style_for(&PAPER_LIGHT);
    let dark = style_for(&PAPER_DARK);
    assert!(dark.highlight_alpha < light.highlight_alpha * 0.5);
    assert!(dark.shadow_alpha > light.shadow_alpha * 2.0);
    assert!(
        dark.alpha_top < light.alpha_top * 0.5,
        "a dark lift stays faint"
    );
}

#[test]
fn off_draws_nothing() {
    for id in ALL_THEMES {
        assert!(!style_for(id.theme()).scaled(GlassLevel::Off).visible());
    }
}

/// The top edge is always at least as opaque as the bottom — that ramp is
/// what makes the sheet look lit from above. Liquid glass's fill is SMOKE
/// (a dark tint over the wallpaper), so for it the same light runs the other
/// way: thinner smoke at the top.
#[test]
fn fill_is_brightest_at_the_top() {
    for id in ALL_THEMES {
        let s = style_for(id.theme());
        if id.theme().liquid.is_some() {
            assert!(s.alpha_top < s.alpha_bottom, "{}: smoke", id.as_str());
            continue;
        }
        assert!(
            s.alpha_top >= s.alpha_bottom,
            "{} inverts the glass gradient",
            id.as_str()
        );
    }
}

/// Level scaling is monotonic, gloss included.
#[test]
fn level_scales_alpha_monotonically() {
    let base = GlassStyle {
        alpha_top: 0.25,
        ..style_for(&CRT_GREEN)
    };
    let at = |l| base.scaled(l);
    let (low, med, high) = (
        at(GlassLevel::Low),
        at(GlassLevel::Medium),
        at(GlassLevel::High),
    );
    assert!(low.alpha_top < med.alpha_top && med.alpha_top < high.alpha_top);
    assert!(low.gloss < med.gloss && med.gloss < high.gloss);
    assert_eq!(at(GlassLevel::Off).gloss, 0.0, "off is off");
}

/// High strength must not push any alpha past opaque.
#[test]
fn high_never_exceeds_opaque() {
    for id in ALL_THEMES {
        let s = style_for(id.theme()).scaled(GlassLevel::High);
        for a in [
            s.alpha_top,
            s.alpha_bottom,
            s.highlight_alpha,
            s.shadow_alpha,
            s.edge_glow,
            s.gloss,
            s.glow,
        ] {
            assert!((0.0..=1.0).contains(&a), "{} alpha {a}", id.as_str());
        }
    }
}

/// A tube's text still reads on its glass (the glass-tube goal, done-means
/// 5). The worst place text sits is the top of a pane hard by its frame, at
/// the High level: the body's top tint plus the edge glow and an etched
/// line, under the gloss's sheen. Every text role clears WCAG there — AAA for the terminal's own
/// text, AA for muted text, and the hint floor the bare page already holds.
/// A tube's window is a faceplate with the desktop faintly through it, so
/// `term_bg` stands in for a dark desktop: the glass and gloss are the part
/// crew owns (crew-app's `tubesheer` holds the text over a white one).
#[test]
fn tube_text_reads_on_its_glass() {
    let over = |under: (u8, u8, u8), top: (u8, u8, u8), a: f32| {
        let m = |u: u8, t: u8| (f32::from(u) + (f32::from(t) - f32::from(u)) * a).round() as u8;
        (m(under.0, top.0), m(under.1, top.1), m(under.2, top.2))
    };
    for id in ALL_THEMES.into_iter().filter(|id| id.theme().is_tube()) {
        let t = id.theme();
        let s = style_for(t).scaled(GlassLevel::High);
        let body = over(
            t.term_bg,
            s.tint,
            (s.alpha_top + s.edge_glow + s.etch).min(1.0),
        );
        let worst = over(body, s.highlight, 0.6 * s.gloss);
        let cr = crate::contrast_ratio;
        for (role, fg, floor) in [
            ("term_fg", t.term_fg, 7.0),
            ("ink", t.ink, 7.0),
            ("text_muted", t.text_muted, 4.5),
            ("hint_fg", t.hint_fg, 2.5),
        ] {
            let got = cr(fg, worst);
            eprintln!("{}: {role} {got:.2}", id.as_str());
            assert!(
                got >= floor,
                "{}: {role} {fg:?} on its glass {worst:?} is {got:.2} (need {floor})",
                id.as_str()
            );
        }
    }
}

/// A tube's raster lives in its glass (the glass-tube goal, done-means 4):
/// no tube lays scanlines over the window any more, the hot phosphors keep a
/// fine raster etched into their glass, and the cool pair runs clear glass.
#[test]
fn tubes_etch_their_raster_into_their_glass() {
    let tubes: Vec<_> = ALL_THEMES
        .into_iter()
        .filter(|id| id.theme().is_tube())
        .collect();
    assert_eq!(
        tubes.len(),
        4,
        "the four phosphors are still tubes: {tubes:?}"
    );
    for id in &tubes {
        let c = id.theme().crt.unwrap();
        assert_eq!(
            c.scanline,
            0.0,
            "{}: scanlines over the window",
            id.as_str()
        );
        assert_eq!(style_for(id.theme()).etch, c.etch, "{}", id.as_str());
    }
    let etched = |id: crate::ThemeId| id.theme().crt.unwrap().etch > 0.0;
    assert!(etched(crate::ThemeId::CrtGreen) && etched(crate::ThemeId::CrtAmber));
    assert!(!etched(crate::ThemeId::CrtBlue) && !etched(crate::ThemeId::CrtViolet));
}

/// When the OS asks for more contrast the glass gives back what lies under
/// the text — the body, its lit edge, the gloss, the etch — by the page
/// wash's own factor, and keeps its rim and halo; asked for nothing, the
/// active style is exactly the theme's.
#[test]
fn high_contrast_quiets_the_glass_under_the_text() {
    let _c = crate::contrast::test_lock();
    let base = style_for(&CRT_GREEN);
    let k = 0.33;
    let q = base.quieted(k);
    for (name, was, now) in [
        ("body", base.alpha_top, q.alpha_top),
        ("edge", base.edge_glow, q.edge_glow),
        ("gloss", base.gloss, q.gloss),
        ("etch", base.etch, q.etch),
    ] {
        assert!((now - was * k).abs() < 1e-6, "{name}: {was} -> {now}");
    }
    assert_eq!(
        (q.highlight_alpha, q.shadow_alpha, q.glow),
        (base.highlight_alpha, base.shadow_alpha, base.glow)
    );
    assert_eq!(base.quieted(1.0), base);
    // And the active style the frame draws really does it.
    let _t = crate::test_guard();
    let was = crate::current_id();
    crate::set_theme(crate::ThemeId::CrtGreen);
    let normal = style();
    crate::contrast::set_high_contrast(true);
    let high = style();
    crate::contrast::set_high_contrast(false);
    crate::set_theme(was);
    assert_eq!(normal, base, "asked for nothing, the style is the theme's");
    assert!(high.alpha_top < normal.alpha_top && high.gloss < normal.gloss);
    assert_eq!(high.highlight_alpha, normal.highlight_alpha);
}

/// A tube's QUIET text reads over a desktop, not just over the black page
/// (2026-10-06, user: "some of the text not appearing properly … we need
/// better color contrast"). The window is see-through, so what sits behind a
/// legend or a "peak" key is the wallpaper through the glass. A dark-grey
/// wallpaper is the stand-in: a phosphor that cannot clear it is darker than
/// the glass it sits on. The reading roles clear AA there, muted text and
/// a legend the UI floor (3:1), and the quiet roles the hint floor the glass
/// already holds them to (2.5:1) — they were near 1.5:1 before.
#[test]
fn tube_quiet_text_reads_over_a_desktop() {
    const DESKTOP: (u8, u8, u8) = (48, 48, 48);
    let over = |under: (u8, u8, u8), top: (u8, u8, u8), a: f32| {
        let m = |u: u8, t: u8| (f32::from(u) + (f32::from(t) - f32::from(u)) * a).round() as u8;
        (m(under.0, top.0), m(under.1, top.1), m(under.2, top.2))
    };
    let mut under = Vec::new();
    for id in ALL_THEMES.into_iter().filter(|id| id.theme().is_tube()) {
        let t = id.theme();
        let s = style_for(t).scaled(GlassLevel::High);
        let glass = over(
            DESKTOP,
            s.tint,
            (s.alpha_top + s.edge_glow + s.etch).min(1.0),
        );
        for (role, fg, floor) in [
            ("term_fg", t.term_fg, 4.5),
            ("ink", t.ink, 4.5),
            ("text_muted", t.text_muted, 3.0),
            ("legend_off", t.legend_off, 3.0),
            ("hint_fg", t.hint_fg, 2.5),
            ("placeholder", t.placeholder, 2.5),
            ("dim", t.dim, 2.5),
        ] {
            let got = crate::contrast_ratio(fg, glass);
            eprintln!("{}: {role} {got:.2} over {glass:?}", id.as_str());
            if got < floor {
                under.push(format!(
                    "{} {role} {fg:?}: {got:.2} (need {floor})",
                    id.as_str()
                ));
            }
        }
    }
    assert!(
        under.is_empty(),
        "under the floor over a desktop through the glass:\n  {}",
        under.join("\n  ")
    );
}

/// Liquid glass shows the wallpaper through its body, so the text on it is
/// only as legible as the frost makes it. Worst case: the wallpaper at a pool's
/// full colour, saturated by the glass (`vibrance`, in gamma space as the
/// shader does it), on a focused card (its smoke thins by 12%) under the
/// gloss's sheen. The body holds the ink at 7:1; the first column of text,
/// which sits a cell inside the rim where the lens has begun to clear the
/// frost, at 4.5:1.
#[test]
fn liquid_text_reads_on_its_glass() {
    for id in ALL_THEMES {
        let t = id.theme();
        let (Some(l), Some(m)) = (t.liquid, t.modern) else {
            continue;
        };
        let s = style_for(t);
        let f = |c: u8| f32::from(c) / 255.0;
        let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        for pole in [m.pole_a, m.pole_b] {
            let (r, g, b) = (f(pole.0), f(pole.1), f(pole.2));
            let y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            let vib = |c: f32| y + (c - y) * l.vibrance;
            for (k, floor) in [(0.0, 7.0), ((1.0f32 - 9.0 / l.bevel).max(0.0).powi(2), 4.5)] {
                let frost = s.alpha_top * (1.0 - 0.12) * (1.0 - l.clear_rim * k);
                let sheen = s.gloss * 0.6;
                let mix = |c: f32, tint: u8| {
                    let body = c.clamp(0.0, 1.0) * (1.0 - frost) + f(tint) * frost;
                    byte(body * (1.0 - sheen) + f(s.highlight.0) * sheen)
                };
                let body = (
                    mix(vib(r), s.tint.0),
                    mix(vib(g), s.tint.1),
                    mix(vib(b), s.tint.2),
                );
                let got = crate::contrast_ratio(t.ink, body);
                assert!(
                    got >= floor,
                    "{}: ink on {body:?} over {pole:?} at lens {k:.2} is {got:.2} (need {floor})",
                    id.as_str()
                );
            }
        }
    }
}
