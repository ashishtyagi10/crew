//! One glance the shots and the contrast sweep share: WEATHER for a Berlin
//! afternoon with its hours, SERVING with two live meters, WAITING ON YOU
//! with a blocked shell, a plan and a running task.
use crate::navglance::{Glance, Serving, Signal};
use crate::usageledger::{WindowStat, Windows};

pub(crate) fn glance() -> Glance {
    Glance {
        weather: crate::navweather::State::Found(crate::navweather::Weather {
            place: "Berlin".into(),
            temp: 24,
            hi: 27,
            lo: 18,
            rain: 10,
            code: 2,
            unit: 'C',
            // A September day: cool at eight, the peak mid-afternoon.
            hours: (0..24)
                .map(|h| {
                    let t = (h as f32 + 8.0 - 15.0) / 24.0 * std::f32::consts::TAU;
                    (22.5 + 4.5 * t.cos()).round() as i32
                })
                .collect(),
            hour: 8,
        }),
        serving: Serving {
            provider: Some("claude-code".into()),
            model: Some("claude-sonnet-5".into()),
            windows: Windows {
                five_h: Some(WindowStat {
                    left_ms: 2 * 3_600_000 + 10 * 60_000,
                    spent: 42,
                    budget: 100,
                }),
                seven_d: Some(WindowStat {
                    left_ms: 3 * 86_400_000 + 4 * 3_600_000,
                    spent: 15,
                    budget: 100,
                }),
            },
        },
        waiting: crate::navglance::rows_from(vec![
            (
                0,
                "smith".to_string(),
                Signal {
                    blocked: false,
                    plan: true,
                    running: 2,
                },
            ),
            (
                2,
                "zsh".to_string(),
                Signal {
                    blocked: true,
                    plan: false,
                    running: 0,
                },
            ),
        ]),
    }
}
