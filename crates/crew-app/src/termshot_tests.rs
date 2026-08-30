//! Off-screen render of a real TERMINAL grid — the surface crew is.
//!
//! Every shot harness in this crate photographs something crew draws itself:
//! the chat transcript, the left nav, the drawn panes, Far, the menus, the
//! todo list. The one thing never in a frame is the thing a terminal is for —
//! a program's own output, arriving as escape sequences and coming out of
//! `crew_term` as `RenderCell`s. The glass/CRT shots put a *Far* pane in the
//! window because a Far pane is easy to build.
//!
//! So the ANSI palette, bold, reverse video, the underline family, the block
//! cursor, wide glyphs on the terminal's own grid and a live selection have
//! all been asserted on as cells and never once looked at.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew term_shot -- --ignored`
use crew_term::{GridSize, HeadlessTerm, TermModel};

use crate::session::to_cellviews;
use crate::shotgpu_tests::{ink, shot_at};

/// A quarter tile, a half tile, and the whole window.
const WIDTHS: [(&str, u32, u32); 3] = [
    ("quarter", 470, 380),
    ("half", 700, 560),
    ("full", 1180, 760),
];

/// The 16 ansi slots as a program actually prints them: the normal row, then
/// the bright row, each swatch on its own background so both the fg and the
/// bg halves of the palette are on the page.
fn palette() -> String {
    let mut s = String::from("\x1b[1mANSI\x1b[0m\r\n");
    for base in [30u8, 90] {
        for i in 0..8u8 {
            s.push_str(&format!("\x1b[{}m ##", base + i));
        }
        s.push_str("\x1b[0m\r\n");
    }
    for base in [40u8, 100] {
        for i in 0..8u8 {
            s.push_str(&format!("\x1b[{}m   ", base + i));
        }
        s.push_str("\x1b[0m\r\n");
    }
    s
}

/// The attribute grammar: every one of these is a separate branch in
/// `celldeco`, and several of them (the underline family especially) are
/// drawn rather than read from the font.
fn attributes() -> String {
    "\x1b[1mbold\x1b[0m \x1b[2mdim\x1b[0m \x1b[3mitalic\x1b[0m \x1b[4munder\x1b[0m \
     \x1b[7mreverse\x1b[0m \x1b[9mstrike\x1b[0m\r\n\
     \x1b[4:2mdouble\x1b[0m \x1b[4:3mcurly\x1b[0m \x1b[4:4mdotted\x1b[0m \
     \x1b[4:5mdashed\x1b[0m \x1b[58:5:9m\x1b[4:3mcolored curly\x1b[0m\r\n\
     \x1b[38;2;255;140;0mtruecolor\x1b[0m \x1b[38;5;213m256-color\x1b[0m \
     \x1b[1;4;31mall three\x1b[0m\r\n"
        .into()
}

/// What a build actually looks like: a bright status word, a dimmed path, a
/// box-drawn frame from a TUI, and a wide-glyph line.
fn session() -> String {
    "\x1b[32m   Compiling\x1b[0m crew-app v0.19.75 (/Users/you/code/crew)\r\n\
     \x1b[33mwarning\x1b[0m: unused variable: \x1b[1m`cols`\x1b[0m\r\n\
     \x1b[1;31merror[E0433]\x1b[0m: cannot find `table` in `md`\r\n\
     \x1b[2m   --> crates/crew-app/src/md/fold.rs:25:28\x1b[0m\r\n\
     \r\n\
     \x1b[36m╭──────────────╮\x1b[0m\r\n\
     \x1b[36m│\x1b[0m tests \x1b[32m2801\x1b[0m ok \x1b[36m│\x1b[0m\r\n\
     \x1b[36m╰──────────────╯\x1b[0m\r\n\
     \r\n\
     日本語のログ行 \x1b[35m→\x1b[0m 全角も一マスに二列\r\n\
     $ \x1b[7m \x1b[0m"
        .into()
}

fn term(body: &str, cols: u16, rows: u16) -> HeadlessTerm {
    let mut t = HeadlessTerm::new(GridSize { cols, rows });
    t.feed(body.as_bytes());
    t
}

fn term_shot(name: &str, body: &str, w: u32, h: u32, select: bool) -> Option<Vec<u8>> {
    shot_at(name, w, h, 13.0, "zsh", |cols, rows, _| {
        let mut t = term(body, cols, rows);
        if select {
            t.sel_start(2, 1, false);
            t.sel_update(cols.saturating_sub(6), 2);
        }
        (to_cellviews(&t.cells(true)), Vec::new())
    })
}

fn sweep(suffix: &str, w: u32, h: u32) -> bool {
    let mut any = false;
    for (kind, body, sel) in [
        ("palette", palette(), false),
        ("attrs", attributes(), false),
        ("session", session(), false),
        ("select", session(), true),
    ] {
        let name = format!("term-{kind}-{suffix}");
        if let Some(px) = term_shot(&name, &body, w, h, sel) {
            any = true;
            let n = ink(&px);
            eprintln!("{name}: {n} ink px");
            assert!(n > 1_000, "{name} is all but blank: {n} ink pixels");
        }
    }
    any
}

/// Every terminal shape at every tile size.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn term_shot_width_sweep() {
    let _g = crate::app::theme_test_guard();
    let mut any = false;
    for (suffix, w, h) in WIDTHS {
        any |= sweep(suffix, w, h);
    }
    if !any {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
    }
}

/// The same grid on a light page and through a green tube. The ansi table is
/// per-theme and the tubes collapse it onto one phosphor, so this is the shot
/// that says whether a program's colours survive the theme it runs under.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn term_shot_themes() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for (suffix, id) in [
        ("light", crew_theme::ThemeId::PaperLight),
        ("crt-green", crew_theme::ThemeId::CrtGreen),
    ] {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        sweep(suffix, 700, 560);
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}
