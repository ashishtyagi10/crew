use crate::glass::{style_for, GlassLevel};
use crate::{contrast_ratio, ALL_THEMES};

fn mix(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> (f32, f32, f32) {
    let m = |x: u8, y: u8| f32::from(x) + (f32::from(y) - f32::from(x)) * t;
    (m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
}

fn byte(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

/// Glass is see-through (the user, 2026-10-08: "I can't see the
/// background"; 2026-10-09: "still not transparent enough"): every liquid
/// theme's window lets most of the desktop through between the panes and
/// a third to a half of it through a pane, and the desktop is the only
/// background — no wallpaper of its own ("I don't think we need gradient
/// colors in glass themes").
#[test]
fn glass_lets_the_desktop_through() {
    for id in ALL_THEMES {
        let Some(l) = id.theme().liquid else { continue };
        let name = id.as_str();
        // No theme paints a wallpaper, so there is nothing to bend: no lens
        // work on a flat page.
        assert_eq!(
            (l.refract, l.blur, l.dispersion),
            (0.0, 0.0, 0.0),
            "{name}: the lens samples a flat page"
        );
        assert!(
            l.window <= 0.35,
            "{name}: the gaps hide {:.2} of the desktop",
            l.window
        );
        assert!(l.body > 0.0 && l.body < 1.0, "{name}: body {:.2}", l.body);
        let cover = l.pane_cover();
        assert!(cover > l.window, "{name}: a pane shows more than a gap");
        // The clear glass hides 0.59; `dark`'s deeper smoke 0.71.
        assert!(
            (0.5..=0.75).contains(&cover),
            "{name}: a pane hides {cover:.2}"
        );
        assert!(
            l.desktop_blur > 0.0 && l.desktop_blur < 80.0,
            "{name}: blur"
        );
    }
}

/// The text on a see-through pane still reads over ANY desktop — black and
/// white, the worst cases — under the clearest frost any level draws (the
/// body's top at that level). The bar is WCAG AA for the ink and the terminal's text, and
/// the UI floor for muted text: seeing the desktop costs some contrast,
/// never legibility.
#[test]
fn glass_text_reads_over_any_desktop() {
    let mut under = Vec::new();
    for id in ALL_THEMES {
        let t = id.theme();
        let Some(l) = t.liquid else { continue };
        // Words stand on the pane and on their own shadow (`text_cover`).
        let cover = l.text_cover(t.crt.map_or(0.0, |c| c.shade));
        for level in [GlassLevel::Low, GlassLevel::Medium, GlassLevel::High] {
            let g = style_for(t).scaled_by(level.liquid_scale());
            {
                // The page is all there is behind the glass: no wallpaper.
                let (r, gr, b) = mix(t.page_bg, t.page_bg, 0.0);
                let y = 0.2126 * r + 0.7152 * gr + 0.0722 * b;
                let vib = |c: f32| byte(y + (c - y) * l.vibrance);
                let wall = (vib(r), vib(gr), vib(b));
                let body = mix(wall, g.tint, g.alpha_top);
                for desk in [0.0_f32, 255.0] {
                    let shown = |c: f32| byte(c * cover + desk * (1.0 - cover));
                    let bg = (shown(body.0), shown(body.1), shown(body.2));
                    for (role, fg, floor) in [
                        ("ink", t.ink, 4.5),
                        ("term_fg", t.term_fg, 4.5),
                        ("text_muted", t.text_muted, 3.0),
                    ] {
                        let got = contrast_ratio(fg, bg);
                        if got < floor {
                            under.push(format!(
                                "{} {} {role} {got:.2} over {bg:?} (desktop {desk})",
                                id.as_str(),
                                level.as_str()
                            ));
                        }
                    }
                }
            }
        }
    }
    assert!(
        under.is_empty(),
        "under the floor:\n  {}",
        under.join("\n  ")
    );
}
