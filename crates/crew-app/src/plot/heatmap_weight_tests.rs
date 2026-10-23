use super::{heat_alpha, idle_ink};
use crew_theme::{contrast_ratio, relative_luminance, ALL_THEMES};

/// `c` at `a` over `bg`.
fn over(c: (u8, u8, u8), a: f32, bg: (u8, u8, u8)) -> (u8, u8, u8) {
    let m = |x: u8, y: u8| (f32::from(y) + (f32::from(x) - f32::from(y)) * a).round() as u8;
    (m(c.0, bg.0), m(c.1, bg.1), m(c.2, bg.2))
}

/// The quietest hour that saw any use (a fiftieth of the peak, a whole tile)
/// against an idle hour's dot. On a page that hides the desktop (the Ink
/// Reader pages, neon's lit room) the tile stands plainly further from the
/// page than the dot, on every ground (UI survey J1: on neon the dot stood
/// 2.8:1 and the tile 1.5:1). Through see-through glass the dot's own floor
/// is the desktop (`an_idle_dot_reads_over_the_glass_on_any_desktop`), so
/// there the tile has to read plainly over a dark desktop.
#[test]
fn the_quietest_used_hour_outweighs_an_idle_one() {
    let _g = crate::app::theme_test_guard();
    let mut light = Vec::new();
    for id in ALL_THEMES {
        crew_theme::set_theme(id);
        let t = id.theme();
        let (dot, a) = idle_ink(t);
        let k = 0.02;
        let ink = crate::modernring::pole_mix(k).unwrap_or(t.accent_default);
        let tile = |g| contrast_ratio(over(ink, heat_alpha(k), g), g);
        match t.liquid.filter(|l| l.holo == 0.0) {
            None => {
                let grounds = match t.liquid {
                    Some(_) => crew_theme::glasslegend::pane_grounds(t),
                    None => vec![t.page_bg],
                };
                for g in grounds {
                    let (idle, used) = (contrast_ratio(over(dot, a, g), g), tile(g));
                    if used <= idle * 1.1 || used < 1.5 {
                        light.push(format!(
                            "{} on {g:?}: used {used:.2}, idle {idle:.2}",
                            id.as_str()
                        ));
                    }
                }
            }
            Some(_) => {
                let dark = (crew_theme::glasslegend::pane_grounds(t).into_iter())
                    .min_by(|x, y| relative_luminance(*x).total_cmp(&relative_luminance(*y)))
                    .expect("a glass pane has grounds");
                let used = tile(dark);
                if used < 2.0 {
                    light.push(format!("{} on {dark:?}: used {used:.2}", id.as_str()));
                }
            }
        }
    }
    assert!(light.is_empty(), "{light:#?}");
}

/// The scale still climbs: a busier hour is never paler.
#[test]
fn heat_climbs_from_its_floor_to_full() {
    assert!((heat_alpha(0.0) - 0.5).abs() < 1e-6);
    assert!((heat_alpha(1.0) - 1.0).abs() < 1e-6);
    let steps: Vec<f32> = (0..=10).map(|i| heat_alpha(i as f32 / 10.0)).collect();
    assert!(steps.windows(2).all(|w| w[0] < w[1]), "{steps:?}");
}
