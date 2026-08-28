//! Off-screen render of the WHOLE crew pane — header, transcript, composer and
//! summary footer in one card, at the widths a tile in the auto-grid actually
//! gets — so the pane a user reads all day can be *looked at* as a pane.
//!
//! Every other chat test asserts on `CellView`s. That is the layout's source of
//! truth and says nothing about what the frame looks like: the left nav shipped
//! three widgets that each passed alone and drew wrong stacked (see
//! `sidebarshot_tests`), and the chat pane stacks far more than the nav does.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew chat_shot -- --ignored`
use crate::chat::ChatPane;
use crate::chatlayout::Message;
use crate::shotgpu_tests::shot_at;
use crew_plugin::{AgentInfo, Plugin};

const H: u32 = 760;

fn msg(sender: &str, text: &str, meta: &str, usage: Option<(u64, u64, u64)>) -> Message {
    Message {
        sender: sender.into(),
        text: text.into(),
        ts: String::new(),
        meta: meta.into(),
        usage,
        expanded: false,
    }
}

/// A session mid-conversation: prose, a heading, a bullet list, inline code, a
/// link, a fenced block, and a reply still streaming in. Every body shape the
/// markdown engine can put on a card, in one transcript, so a width sweep
/// exercises all of them at once.
fn live_pane() -> ChatPane {
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    let mut p = ChatPane::new(plugin, "crew".into());
    p.connected = true;
    p.agents = vec![
        AgentInfo {
            name: "smith".into(),
            role: "lead".into(),
            model: "claude-opus-5".into(),
        },
        AgentInfo {
            name: "scout".into(),
            role: "search".into(),
            model: "claude-haiku-4-5".into(),
        },
    ];
    p.messages = vec![
        msg(
            "user",
            "why is the sidebar chart a smear at two rows?",
            "",
            None,
        ),
        msg(
            "smith",
            "Short answer: it is drawn on a canvas coarser than the screen.\n\n\
             ## What happens\n\n\
             - `plot::Canvas` samples an inside/outside predicate on a 3×3 grid\n\
             - at `SUB = 4` that grid is *half* the device resolution\n\
             - a stroke thinner than one canvas pixel misses all nine samples\n\n\
             The fix is a signed distance field — see `plot/sdf.rs` and the note in \
             [the design doc](https://example.invalid/crew/design).\n\n\
             ```rust\n\
             let d = sdf::arc(p, r, half_w, a0, a1);\n\
             let cov = (0.5 - d * scale).clamp(0.0, 1.0);\n\
             ```\n\n\
             Sampling at `SUB = 8` is where the stepping stops; 16 is not visibly \
             better at twice the quads.",
            "6.1s",
            Some((3120, 486, 21400)),
        ),
        msg(
            "user",
            "and the ring's track? it vanishes on paper light",
            "",
            None,
        ),
        msg(
            "scout",
            "`border_normal` reads **1.28** against `PaperLight` — under the 1.3 floor \
             a scale needs. `palette::accent` grew an `enforced` floor for exactly this.",
            "1.4s",
            Some((880, 96, 4100)),
        ),
    ];
    p.streaming.push(msg(
        "smith",
        "Rolling that floor out to the dial's ticks now — the scale is the finest thing",
        "",
        None,
    ));
    p.tokens = 41_820;
    p.tok_in = 36_140;
    p.tok_out = 5_680;
    p.cost_microusd = 214_000;
    p.turns = 4;
    p.git_branch = Some("feat/chat-shot".into());
    p.cwd = Some("~/code/crew".into());
    p.running_tasks = vec![7, 9];
    p
}

fn chat_shot(name: &str, w: u32) -> Option<Vec<u8>> {
    let pane = live_pane();
    shot_at(name, w, H, 13.0, "crew", |cols, rows, aspect| {
        crate::chatview::art(&pane, cols, rows, aspect)
    })
}

/// The pane at the widths the auto-grid hands it: a quarter tile on a laptop,
/// a half tile, and the whole window. A transcript that only reads at one of
/// them reads at none the user actually has.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn chat_shot_width_sweep() {
    let _g = crate::app::theme_test_guard();
    for (name, w) in [
        ("chat-quarter", 470),
        ("chat-half", 700),
        ("chat-full", 1180),
    ] {
        let Some(px) = chat_shot(name, w) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 4000, "{name} drew");
    }
}

/// The same pane on a light page and on a phosphor tube: the two backgrounds
/// where a muted role colour or a dimmed code-block field can disappear.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn chat_shot_themes() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for (name, id) in [
        ("chat-light", crew_theme::ThemeId::PaperLight),
        ("chat-crt-green", crew_theme::ThemeId::CrtGreen),
    ] {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        let Some(px) = chat_shot(name, 700) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 4000, "{name} drew");
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}
