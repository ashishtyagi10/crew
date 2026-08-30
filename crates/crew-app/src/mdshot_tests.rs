//! Off-screen render of the markdown an agent actually replies with.
//!
//! `chatshot_tests` shoots one transcript: prose, a heading, a bullet list,
//! inline code, a link and a fenced block. That is the half of the grammar the
//! engine was written against. The other half — tables, nested and ordered
//! lists, task lists, block quotes, rules, an unbreakable URL, CJK — has been
//! asserted on as `MdLine`s and never once looked at on a card, and a table is
//! exactly the shape that survives one width and falls apart at another.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew md_shot -- --ignored`
use crate::chat::ChatPane;
use crate::chatlayout::Message;
use crate::shotgpu_tests::shot_at;
use crew_plugin::Plugin;

const H: u32 = 900;

/// A quarter tile, a half tile, and the whole window.
const WIDTHS: [(&str, u32); 3] = [("quarter", 470), ("half", 700), ("full", 1180)];

fn msg(text: &str) -> Message {
    Message {
        sender: "smith".into(),
        text: text.into(),
        ts: String::new(),
        meta: String::new(),
        usage: None,
        expanded: false,
    }
}

fn pane(body: &str) -> ChatPane {
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    let mut p = ChatPane::new(plugin, "crew".into());
    p.connected = true;
    p.messages = vec![msg(body)];
    p
}

/// A comparison table — the shape an agent reaches for whenever it is asked
/// "which one", and the one shape whose columns cannot simply wrap.
const TABLE: &str = "Here is how the three compare.\n\n\
| renderer | glyph cache | ligatures | notes |\n\
|---|---:|:--:|---|\n\
| ghostty | atlas | yes | metal + opengl |\n\
| alacritty | atlas | no | the smallest of the three |\n\
| wezterm | shaper | yes | lua config, multiplexer built in |\n\n\
The middle column is right-aligned and the third centred, so a table that \
ignores its alignment row is visible at a glance.";

/// Lists inside lists, ordered and unordered, plus a task list — the shape a
/// plan comes back in. Written flush-left on purpose: a `\`-continuation in a
/// Rust literal eats the next line's leading whitespace, which is exactly the
/// indentation a nested list IS.
const LISTS: &str = "\
## the plan

1. read the frame
   1. `build_frame` hands the pane its rect
   2. the card is pushed with a legend
2. lay the interior out
   - every band at its floor
     - and the floor is a `const`
   - then the slack goes to the histories
3. draw

- [x] shoot the surface
- [ ] look at the shot
- [ ] fix what it showed

* a second list, started with a star
* so the bullet glyph is the engine's, not the source's
";

/// The things that break a wrapper: a quote, a rule, an unbreakable URL, a
/// long token with no spaces, and wide glyphs.
const EDGES: &str = "> A pane that draws nothing at one size is blank on\n\
> somebody's screen.\n\
>\n\
> — the panel sweep\n\n\
---\n\n\
The link is <https://example.invalid/crew/very/long/path/that/will/not/fit/in/a/quarter/tile?with=query&and=more> \
and the identifier is `crate::todopane::render::first_line_max_before_the_chips`.\n\n\
日本語の見出しと **太字** を混ぜた行、それに `code` も。\n\n\
Ends with a trailing hard break.  \nAnd the line after it.";

fn md_shot(name: &str, body: &str, w: u32) -> Option<Vec<u8>> {
    let p = pane(body);
    shot_at(name, w, H, 13.0, "crew", |cols, rows, aspect| {
        crate::chatview::art(&p, cols, rows, aspect)
    })
}

fn sweep(suffix: &str, w: u32) -> bool {
    let mut any = false;
    for (kind, body) in [("table", TABLE), ("lists", LISTS), ("edges", EDGES)] {
        let name = format!("md-{kind}-{suffix}");
        if let Some(px) = md_shot(&name, body, w) {
            any = true;
            let n = crate::shotgpu_tests::ink(&px);
            eprintln!("{name}: {n} ink px");
            assert!(n > 3_000, "{name} is all but blank: {n} ink pixels");
        }
    }
    any
}

/// Every markdown shape at every tile width.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn md_shot_width_sweep() {
    let _g = crate::app::theme_test_guard();
    let mut any = false;
    for (suffix, w) in WIDTHS {
        any |= sweep(suffix, w);
    }
    if !any {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
    }
}

/// The same shapes on a light page and through a green tube: the quote ink,
/// the code field's tint and the table's rules are all theme roles.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn md_shot_themes() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for (suffix, id) in [
        ("light", crew_theme::ThemeId::PaperLight),
        ("crt-green", crew_theme::ThemeId::CrtGreen),
    ] {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        sweep(suffix, 700);
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}
