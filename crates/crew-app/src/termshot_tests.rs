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

/// Marks laid ON full-width text: a selection, the block cursor, an
/// underline and a painted background all have to cover BOTH columns the
/// character occupies.
const WIDE: &str = "\x1b[4munderlined 日本語 text\x1b[0m\r\n\
     \x1b[44m painted 全角 background \x1b[0m\r\n\
     plain ascii for scale\r\n";

/// A full-screen TUI on the alternate screen: the shape `less`, `htop`,
/// `lazygit` and every agent CLI's picker put on a pane.
const ALTSCREEN: &str = "\x1b[?1049h\x1b[2J\x1b[H\
     \x1b[1;36m FILES \x1b[0m\x1b[2m  ~/code/crew\x1b[0m\r\n\
     \x1b[2m\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\x1b[0m\r\n\
     \x1b[42;30m src/main.rs \x1b[0m\r\n\
     \x1b[32m src/lib.rs\x1b[0m\r\n\
     \x1b[34m docs/\x1b[0m\r\n\
     \x1b[2m 3 entries \u{00b7} q to quit\x1b[0m";

/// A pane with plenty to reach: the shape hint mode is for.
const HINTY: &str = "\x1b[2m$\x1b[0m cargo test\r\n\
     \x1b[1;31merror\x1b[0m: cannot find `table` in crates/crew-app/src/md/fold.rs:25\r\n\
     \x1b[33mwarning\x1b[0m: unused import in src/main.rs:7\r\n\
     see https://doc.rust-lang.org/error_codes/E0433.html\r\n\
     \x1b[2mHEAD is now at 9f3ab1c7 the caret leaves a wake\x1b[0m\r\n\
     docs/CREW.md README.md Cargo.toml\r\n";

/// Lines with URLs in them, which crew tints and rules itself.
const LINKS: &str = "see https://example.invalid/crew/docs for the rest\r\n\
     mirror at http://localhost:8080/status (no tls)\r\n\
     not a link: example.invalid/crew\r\n";

fn term(body: &str, cols: u16, rows: u16) -> HeadlessTerm {
    let mut t = HeadlessTerm::new(GridSize { cols, rows });
    t.feed(body.as_bytes());
    t
}

/// What crew lays over a pane's own cells each frame, after the terminal has
/// had its say: a live selection, `/find` match washes, URL tinting.
#[derive(Clone, Copy, Default)]
struct Overlay {
    select: bool,
    find: Option<&'static str>,
    links: bool,
    /// A caret wake mid-fade: the cell the cursor came from and the cell it is
    /// in now (see [`crate::cursortrail`]). The one thing on a terminal pane
    /// drawn on the paint layer rather than as cells, so it is also the one
    /// thing a cell assertion cannot see.
    wake: Option<((u16, u16), (u16, u16))>,
    /// A picture the program sent, in the graphics protocol's own form — fed
    /// as bytes, placed by the terminal, decoded on a worker, drawn on the
    /// paint layer. The whole path, in one frame.
    picture: bool,
    /// Hint mode over this pane's output (Cmd+E): every URL, file reference
    /// and hash wearing the letter that reaches it.
    hints: bool,
}

/// A small PNG with a shape in it, written the way a program would have.
fn picture_png(w: u32, h: u32) -> Vec<u8> {
    let img = image::RgbaImage::from_fn(w, h, |x, y| {
        let (fx, fy) = (x as f32 / w as f32, y as f32 / h as f32);
        let r = ((fx - 0.5).powi(2) + (fy - 0.5).powi(2)).sqrt();
        match r < 0.34 {
            true => image::Rgba([250, 236, 200, 255]),
            false => image::Rgba([(fx * 220.0) as u8, 40, (fy * 220.0) as u8, 255]),
        }
    });
    let mut buf = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut buf, image::ImageFormat::Png)
        .expect("encode");
    buf.into_inner()
}

/// The escape sequence a program writes to show `png`.
fn graphics_seq(png: &[u8], cols: u16, rows: u16) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut b64 = String::new();
    for c in png.chunks(3) {
        let n = (u32::from(c[0]) << 16)
            | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
            | u32::from(*c.get(2).unwrap_or(&0));
        for i in 0..4 {
            match i <= c.len() {
                true => b64.push(A[(n >> (18 - 6 * i)) as usize & 63] as char),
                false => b64.push('='),
            }
        }
    }
    format!("\x1b_Ga=T,f=100,c={cols},r={rows};{b64}\x1b\\")
}

fn term_shot(name: &str, body: &str, w: u32, h: u32, o: Overlay) -> Option<Vec<u8>> {
    shot_at(name, w, h, 13.0, "zsh", |cols, rows, _| {
        let mut t = term(body, cols, rows);
        if o.select {
            t.sel_start(2, 1, false);
            t.sel_update(cols.saturating_sub(6), 2);
        }
        let mut cells = to_cellviews(&t.cells(true));
        if let Some(term) = o.find {
            crate::findhl::highlight(&mut cells, term, cols, rows);
        }
        if o.links {
            crate::linkhl::colorize(&mut cells, cols, rows);
        }
        let mut paint = o
            .wake
            .map(|(from, at)| {
                let mut trail = crate::cursortrail::Trail::default();
                trail.observe(Some(from), 1_000);
                trail.observe(Some(at), 1_000);
                trail.paint(
                    1_030,
                    crew_theme::readable::cursor(crew_theme::theme(), true),
                )
            })
            .unwrap_or_default();
        if o.hints {
            let rows_of_text = crate::gridrows::grid_lines(&cells, cols, rows);
            if let Some(h) = crate::hints::Hints::scan(&rows_of_text) {
                h.mark(&mut cells);
            }
        }
        if o.picture {
            let mut store = crate::termimg::TermImages::default();
            store.collect(t.take_images());
            // The decode is on a worker, as it is in the app; a shot has to
            // wait for it rather than photograph a pane mid-load.
            for _ in 0..400 {
                if store.poll() || !store.loading() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            paint.extend(store.paint(t.history_lines(), t.display_offset(), cols, rows, 2.1));
        }
        (cells, paint)
    })
}

fn sweep(suffix: &str, w: u32, h: u32) -> bool {
    let mut any = false;
    let sel = Overlay {
        select: true,
        ..Default::default()
    };
    let find = |t| Overlay {
        find: Some(t),
        ..Default::default()
    };
    let links = Overlay {
        links: true,
        ..Default::default()
    };
    // What `icat` puts on a pane: a line of text, a picture, a line of text.
    let pic_body = format!(
        "\x1b[36m$\x1b[0m plot --last 7d\r\n{}\r\n\x1b[2msaved to ~/plot.png\x1b[0m\r\n",
        graphics_seq(&picture_png(240, 120), 24, 6)
    );
    let wake = |from, at| Overlay {
        wake: Some((from, at)),
        ..Default::default()
    };
    for (kind, body, o) in [
        ("palette", palette(), Overlay::default()),
        ("attrs", attributes(), Overlay::default()),
        ("session", session(), Overlay::default()),
        ("select", session(), sel),
        ("wide", WIDE.to_string(), sel),
        ("alt", ALTSCREEN.to_string(), Overlay::default()),
        ("find", session(), find("crew")),
        ("find-wide", WIDE.to_string(), find("\u{5168}\u{89d2}")),
        ("links", LINKS.to_string(), links),
        (
            "picture",
            pic_body.clone(),
            Overlay {
                picture: true,
                ..Default::default()
            },
        ),
        (
            "hints",
            HINTY.to_string(),
            Overlay {
                hints: true,
                ..Default::default()
            },
        ),
        ("wake", session(), wake((14, 10), (2, 10))),
        ("wake-jump", session(), wake((30, 1), (2, 6))),
    ] {
        let name = format!("term-{kind}-{suffix}");
        if let Some(px) = term_shot(&name, &body, w, h, o) {
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
