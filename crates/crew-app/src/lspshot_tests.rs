//! Off-screen render of the viewer wearing a language server's verdict — the
//! margin marks, the curly underlines, and the legend that counts them.
//!
//! This surface has never been looked at. It was built with unit tests over
//! the span arithmetic (`lspdeco`, `lspgutter`) and a legend test, which say
//! that a span lands on the right CELLS; none of them say what the pane looks
//! like with a server's real output on it — three diagnostics of two
//! severities, one of them on a line long enough to wrap, at the widths a
//! tile actually gets.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew lsp_shot -- --ignored --nocapture`
use crew_lsp::{Diagnostic, Position, Range, Severity};

use crate::shotgpu_tests::shot_at;
use crate::viewpane::detect::Format;
use crate::viewpane::load::Loaded;
use crate::viewpane::{LoadState, ViewPane};

/// A file with something wrong on three of its lines, one of them past the
/// width of a tile so the underline has to survive a wrap.
const RUST: &str = "\
use crate::plot::sdf;

pub(crate) fn coverage(d: f32, scale: f32) -> f32 {
    let inside = sdf::arc(d, scale, half_w, a0, a1) + unresolved_helper(d) * scale;
    (0.5 - inside * scale).clamp(0.0, 1.0)
}

fn unused(p: (f32, f32)) -> f32 {
    p.0
}
";

fn diag(line: u32, start: u32, end: u32, severity: Severity, message: &str) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position {
                line,
                character: start,
            },
            end: Position {
                line,
                character: end,
            },
        },
        severity,
        message: message.into(),
        source: Some("rust-analyzer".into()),
    }
}

fn diags() -> Vec<Diagnostic> {
    vec![
        diag(
            3,
            60,
            78,
            Severity::Error,
            "cannot find function `unresolved_helper` in this scope",
        ),
        diag(3, 21, 24, Severity::Warning, "unused variable: `arc`"),
        diag(
            7,
            3,
            9,
            Severity::Warning,
            "function `unused` is never used",
        ),
    ]
}

fn pane(with_diags: bool) -> ViewPane {
    let mut p = ViewPane::open(std::env::temp_dir().join("coverage.rs"));
    p.state = LoadState::Ready {
        format: Format::Code { lang: "rust" },
        loaded: Loaded {
            text: RUST.into(),
            truncated: None,
            meta: None,
            image: None,
        },
    };
    if with_diags {
        p.lsp = crate::viewpane::lspjob::Lsp::On(diags());
    }
    p
}

fn lsp_shot(name: &str, p: &ViewPane, w: u32) -> Option<Vec<String>> {
    let mut dumped = Vec::new();
    let px = shot_at(name, w, 300, 13.0, "coverage.rs", |cols, rows, aspect| {
        let (cells, paint) = p.art(cols, rows, aspect);
        dumped = crate::goalshot_tests::dump(&cells, cols, rows);
        eprintln!("--- {name} {cols}x{rows}");
        for l in &dumped {
            eprintln!("|{l}");
        }
        (cells, paint)
    })?;
    assert!(crate::shotgpu_tests::ink(&px) > 2000, "{name} drew");
    Some(dumped)
}

/// The margin and the underlines, at a whole window and at a tile.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn lsp_shot_marked_and_underlined() {
    let _g = crate::app::theme_test_guard();
    let p = pane(true);
    for (name, w) in [("lsp-wide", 900u32), ("lsp-tile", 460)] {
        let Some(rows) = lsp_shot(name, &p, w) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        let all = rows.join("\n");
        assert!(all.contains('\u{25cf}'), "{name}: no error mark:\n{all}");
        assert!(all.contains('\u{25b2}'), "{name}: no warning mark:\n{all}");
    }
}

/// The same file with the server never asked: the margin is not there at all,
/// and the code keeps the columns it would have taken.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn lsp_shot_without_a_server() {
    let _g = crate::app::theme_test_guard();
    let p = pane(false);
    let Some(rows) = lsp_shot("lsp-none", &p, 460) else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    let all = rows.join("\n");
    assert!(!all.contains('\u{25cf}'), "a mark with no server:\n{all}");
}
