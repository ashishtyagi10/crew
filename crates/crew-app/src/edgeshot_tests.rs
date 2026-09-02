//! Off-screen render of four small surfaces that appear in the last row or
//! over a form and had never been in a frame: the `/far` make-folder prompt
//! that replaces the function-key bar, the settings pane's font dropdown, the
//! viewer's `/` search line, and the viewer's card for a file it cannot
//! render.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew edge_shot -- --ignored --nocapture`
use crate::config::CrewConfig;
use crate::farpane::{FarPane, Prompt, PromptKind};
use crate::goalshot_tests::dump;
use crate::settingspane::SettingsPane;
use crate::shotgpu_tests::shot_at;
use crate::viewpane::detect::{Format, Opaque};
use crate::viewpane::load::{FileMeta, Loaded};
use crate::viewpane::{LoadState, ViewPane};

fn shot(
    name: &str,
    legend: &str,
    w: u32,
    h: u32,
    cells: impl Fn(u16, u16, f32) -> (Vec<crew_render::CellView>, Vec<crew_render::Paint>),
) -> Option<Vec<String>> {
    let mut dumped = Vec::new();
    shot_at(
        &format!("edge-{name}"),
        w,
        h,
        13.0,
        legend,
        |cols, rows, aspect| {
            let (c, p) = cells(cols, rows, aspect);
            dumped = dump(&c, cols, rows);
            eprintln!("--- edge-{name} {cols}x{rows}");
            for l in &dumped {
                eprintln!("|{l}");
            }
            (c, p)
        },
    )?;
    Some(dumped)
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn edge_shot_far_prompt() {
    let _g = crate::app::theme_test_guard();
    let mut p = FarPane::new(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    p.prompt = Some(Prompt {
        kind: PromptKind::MkDir,
        input: "shots".into(),
    });
    let Some(rows) = shot("far-prompt", "far", 900, 300, |c, r, _| {
        (p.cells(c, r), Vec::new())
    }) else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    assert!(
        rows.last().is_some_and(|l| l.contains("Create folder")),
        "{rows:?}"
    );
    p.prompt = Some(Prompt {
        kind: PromptKind::MkDir,
        input: "screenshots-from-the-second-of-september-before-the-release".into(),
    });
    shot("far-prompt-tile", "far", 420, 260, |c, r, _| {
        (p.cells(c, r), Vec::new())
    });
}

fn settings() -> SettingsPane {
    let cfg = CrewConfig {
        theme: Some("paper-dark".into()),
        font_family: Some("Lilex".into()),
        ..Default::default()
    };
    let mut p = SettingsPane::new(
        cfg,
        [
            "Lilex",
            "MonoLisa",
            "SF Mono",
            "JetBrains Mono",
            "Fira Code",
        ]
        .iter()
        .map(|s| (*s).to_string())
        .collect(),
    );
    p.family_open = true;
    p.family_sel = 2;
    p
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn edge_shot_settings_dropdown() {
    let _g = crate::app::theme_test_guard();
    let p = settings();
    for (name, w, h) in [
        ("dropdown", 700, 560),
        ("dropdown-short", 700, 300),
        ("dropdown-narrow", 380, 560),
    ] {
        let Some(rows) = shot(name, "settings", w, h, |c, r, _| {
            (p.cells(c, r), Vec::new())
        }) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(
            rows.iter().any(|l| l.contains("fonts")),
            "{name}: the dropdown is open but not drawn: {rows:?}"
        );
    }
}

const TEXT: &str = "\
//! The signed distance field the dial's scale is drawn from.
use crate::plot::sdf;

/// Coverage for one sample: negative is inside, positive is out.
pub(crate) fn coverage(d: f32, scale: f32) -> f32 {
    (0.5 - d * scale).clamp(0.0, 1.0)
}

fn arc(p: (f32, f32), r: f32, half_w: f32, a0: f32, a1: f32) -> f32 {
    sdf::arc(p, r, half_w, a0, a1)
}
";

fn viewer(format: Format, text: &str, meta: Option<FileMeta>) -> ViewPane {
    let mut p = ViewPane::open(std::env::temp_dir().join("edge.rs"));
    p.state = LoadState::Ready {
        format,
        loaded: Loaded {
            text: text.into(),
            truncated: None,
            meta,
            image: None,
        },
    };
    p
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn edge_shot_viewer_search_and_opaque() {
    let _g = crate::app::theme_test_guard();
    let mut p = viewer(Format::Code { lang: "rust" }, TEXT, None);
    let lines: Vec<&str> = TEXT.lines().collect();
    let hits = crate::viewpane::search::find_matches(&lines, "sdf");
    p.search = Some(crate::viewpane::search::Search::new("sdf".into(), hits));
    shot("search", "edge.rs", 480, 300, |c, r, a| p.art(c, r, a));
    let mut typing = crate::viewpane::search::Search::new("sd".into(), vec![]);
    typing.typing = true;
    p.search = Some(typing);
    shot("search-typing", "edge.rs", 480, 300, |c, r, a| {
        p.art(c, r, a)
    });
    p.search = Some(crate::viewpane::search::Search::new("zebra".into(), vec![]));
    shot("search-none", "edge.rs", 480, 300, |c, r, a| p.art(c, r, a));

    let meta = FileMeta {
        size: 4_718_592,
        modified: Some(std::time::SystemTime::now() - std::time::Duration::from_secs(3 * 3600)),
    };
    let p = viewer(
        Format::Opaque {
            why: Opaque::Binary,
        },
        "",
        Some(meta),
    );
    for (name, w) in [("opaque", 700), ("opaque-narrow", 300)] {
        shot(name, "crew.dylib", w, 200, |c, r, a| p.art(c, r, a));
    }
}
