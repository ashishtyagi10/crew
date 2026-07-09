mod altscroll;
mod anim;
mod app;
mod askbar;
mod attention;
mod boxdraw;
mod chat;
mod chatbody;
mod chatchips;
mod chatcompact;
mod chatcomplete;
mod chatempty;
mod chatevents;
mod chatexport;
mod chatflow;
mod chathdr;
mod chatinput;
mod chatkeys;
mod chatlayout;
mod chatmd;
mod chatmention;
mod chatmsgs;
mod chatpalette;
mod chatplace;
mod chatpulse;
mod chatroster;
mod chatscroll;
mod chatspawn;
mod chattheme;
mod chattime;
mod chatview;
mod chatwidth;
mod chords;
pub mod chrome;
mod clickopen;
mod clipboard;
mod clock;
mod cmdcheck;
mod cmdmenu;
pub mod config;
mod ctxlimit;
mod cwd;
mod detach;
mod dispatch;
mod dump;
mod editpane;
mod envexpand;
mod events;
mod farpane;
mod fileindex;
mod findhl;
mod fontcmd;
mod gauges;
mod git;
pub(crate) mod grid;
mod gridrows;
mod gridsel;
mod handler;
mod help;
mod history;
mod histsearch;
mod hit;
mod host;
pub(crate) mod inputbar;
mod inputbar_render;
mod inputkeys;
mod keys;
mod layout;
mod linkhl;
mod load;
mod md;
mod mdcache;
mod mdkeys;
mod mdpane;
mod mdpane_view;
mod minstrip;
mod navcard;
mod navlog;
mod net;
mod notify;
mod openurl;
mod palette;
mod pane;
mod panecard;
mod panelist;
mod panemanage;
mod paneview;
mod pathcomplete;
mod pathexpand;
mod poll;
mod procname;
mod progress;
mod quit;
mod render;
mod restart;
mod route;
mod runpane;
mod scroll;
mod search;
mod select;
mod selfupdate;
mod session;
mod settingspane;
mod spark;
mod spawn;
mod spawnmd;
pub mod stats;
mod statspane;
mod status;
mod suggest;
mod swarm;
mod swarmpane;
mod termwrite;
mod toggles;
mod tui;
mod update;
mod updatecard;
mod updatefetch;
mod welcome;
mod welcomeart;
mod windowtitle;

fn main() -> anyhow::Result<()> {
    // When the `/crew` pane spawns this binary as its multi-agent broker (a
    // re-exec of `crew` with this flag), run the JSON-line broker loop and exit
    // before any GUI initialization. This means `/crew` works wherever `crew`
    // is installed, with no separate plugin binary to ship.
    if std::env::args().skip(1).any(|a| a == "--broker-plugin") {
        return crew_plugin::run_broker_stdio();
    }
    // `/update` re-execs this binary with `--self-update` inside a terminal pane:
    // download the latest release over ourselves, show a progress bar, and exit.
    if std::env::args().skip(1).any(|a| a == "--self-update") {
        return selfupdate::run();
    }
    // `--list-fonts`: print every monospace family the font picker offers
    // (faces flagged monospaced + name-matched coding fonts), then exit — the
    // quick way to check a newly installed font is visible to Crew.
    if std::env::args().skip(1).any(|a| a == "--list-fonts") {
        for name in crew_render::list_monospace_families() {
            println!("{name}");
        }
        return Ok(());
    }
    // Detached launch is the default: re-launch in a new session (detached from
    // this terminal) and exit the parent, so closing the launching shell doesn't
    // SIGHUP the GUI. `--no-detach` / `--foreground` keeps crew attached. The
    // re-launched child sets CREW_DETACHED, so it falls through to the GUI.
    if detach::should_detach() && !detach::is_detached_child() {
        return detach::relaunch_detached();
    }
    // Only the GUI path forks/reads a login shell to seed PATH — CLI modes
    // above (broker/self-update/list-fonts/detach re-exec) return before
    // this line, so they never pay for a shell they don't use.
    cmdcheck::init_shell_path();
    handler::run()
}
