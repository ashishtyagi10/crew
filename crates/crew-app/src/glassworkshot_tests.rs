//! The glass themes over the two panes crew is used for most: agent smith
//! mid-conversation and a shell with a day's output in it. The glass survey
//! shoots the panes crew draws itself (todo, settings, the dashboard); these
//! two are where the reading happens, and neither had been looked at sheer.
//!
//! Composite over a dark and a light desktop to judge it, as the survey says.
//!
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew glass_work_shot -- --ignored`
use crate::layout::Rect;
use crate::pane::{Pane, PaneContent, TermPane};
use crew_term::{GridSize, PtyTerm, TermModel};

/// A prompt, `git status`, `ls`, a build with a warning and an error, and
/// the sixteen ANSI slots — what a shell pane actually shows.
const SHELL: &str = "\x1b[1;34m~/code/crew\x1b[0m \x1b[35mmain\x1b[0m \x1b[32m\u{276f}\x1b[0m git status -s\r\n\
     \x1b[31m M\x1b[0m crates/crew-app/src/glass.rs\r\n\
     \x1b[32mA \x1b[0m crates/crew-theme/src/glassink.rs\r\n\
     \x1b[31m??\x1b[0m notes.md\r\n\
     \x1b[1;34m~/code/crew\x1b[0m \x1b[35mmain\x1b[0m \x1b[32m\u{276f}\x1b[0m ls\r\n\
     \x1b[1;34mcrates\x1b[0m  \x1b[1;34mdocs\x1b[0m  Cargo.toml  README.md  \x1b[1;32mrel.sh\x1b[0m\r\n\
     \x1b[1;34m~/code/crew\x1b[0m \x1b[35mmain\x1b[0m \x1b[32m\u{276f}\x1b[0m cargo test\r\n\
     \x1b[1;32m   Compiling\x1b[0m crew-app v0.26.23 (/Users/you/code/crew)\r\n\
     \x1b[1;33mwarning\x1b[0m: unused variable: \x1b[1m`cols`\x1b[0m\r\n\
     \x1b[1;31merror[E0433]\x1b[0m: cannot find `table` in `md`\r\n\
     \x1b[2m   --> crates/crew-app/src/md/fold.rs:25:28\x1b[0m\r\n\
     test result: \x1b[32mok\x1b[0m. 4048 passed; \x1b[31m0 failed\x1b[0m; \x1b[33m133 ignored\x1b[0m\r\n\
     \x1b[30m##\x1b[31m##\x1b[32m##\x1b[33m##\x1b[34m##\x1b[35m##\x1b[36m##\x1b[37m##\x1b[0m \
     \x1b[90m##\x1b[91m##\x1b[92m##\x1b[93m##\x1b[94m##\x1b[95m##\x1b[96m##\x1b[97m##\x1b[0m\r\n\
     \x1b[1;34m~/code/crew\x1b[0m \x1b[35mmain\x1b[0m \x1b[32m\u{276f}\x1b[0m ";

/// A pane around `content`, as the app pushes one.
fn pane(content: PaneContent, dir: Option<&str>) -> Pane {
    Pane {
        glide: crate::glide::Glide::default(),
        content,
        grid: GridSize { cols: 80, rows: 24 },
        rect: Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: None,
        name: None,
        dir: dir.map(Into::into),
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: 0,
    }
}

/// A shell pane holding [`SHELL`]. The child is `cat`, which prints nothing
/// of its own, so the frame shows exactly the bytes fed here.
fn shell() -> Pane {
    let tmp = std::env::temp_dir();
    let grid = GridSize { cols: 80, rows: 24 };
    let mut pty = PtyTerm::spawn_in(grid, "/bin/cat", &[], Some(&tmp)).unwrap();
    pty.feed(SHELL.as_bytes());
    let input = pty.writer();
    let term = TermPane {
        pty,
        input,
        cmd: None,
        cmd_since: None,
        tail: Default::default(),
        read_at: 0,
        spans: Default::default(),
        trail: Default::default(),
        images: Default::default(),
    };
    pane(
        PaneContent::Terminal(Box::new(term)),
        Some("/Users/you/code/crew"),
    )
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn glass_work_shot() {
    use crew_theme::ThemeId;
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for id in [ThemeId::GlassClear, ThemeId::GlassNight] {
        let sheer = crate::tubesheer::sheer(1.0, id.theme());
        let prep = |app: &mut crate::app::CrewApp| {
            let smith = crate::chatshot_tests::live_pane();
            app.panes.push(pane(PaneContent::Chat(smith), None));
            app.panes.push(shell());
            app.focused = 0;
            app.input.cwd = "/Users/you/code/crew".into();
        };
        let name = format!("work-{}", id.as_str());
        let shot = crate::crtglassshot_tests::window_with(
            &name,
            id,
            Default::default(),
            sheer,
            &[],
            &prep,
        );
        if shot.is_none() {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        }
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}
