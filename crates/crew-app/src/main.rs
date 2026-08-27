//! `crew` — the GUI, and every console mode that must answer before it.
//!
//! **GUI subsystem** (Windows): a console-subsystem binary is handed a console
//! window by the OS before `main` runs, so every Start-menu launch flashed a
//! black window. Declaring the GUI subsystem means one is never created. The
//! console modes below get their stdio back through [`wincon::attach_to_parent`]
//! — see that module for what the choice costs.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod activitylog;
mod altscroll;
mod anim;
mod app;
mod applog;
mod appregister;
mod askaddr;
mod askbar;
mod askcast;
mod askclient;
mod askpump;
mod askrender;
mod askroute;
mod askwait;
mod attention;
mod autoupdate;
mod blocked;
mod bordermarks;
mod boxdraw;
mod channel;
mod charrain;
mod chat;
mod chataction;
mod chatbody;
mod chatcompact;
mod chatcomplete;
mod chatempty;
mod chatevents;
mod chatexport;
mod chatfind;
mod chatflow;
mod chatfold;
mod chatfont;
mod chathdr;
mod chathistory;
mod chathistsearch;
mod chatink;
mod chatinput;
mod chatkeys;
mod chatkeystore;
mod chatlayout;
mod chatmd;
mod chatmention;

mod chatmsgs;
mod chatpalette;
mod chatplace;
mod chatprog;
mod chatpulse;
mod chatqueue;
mod chatroster;
mod chatscroll;
mod chatsettle;
mod chatspawn;
mod chatsummary;
mod chatswarm;
mod chatswarmview;
mod chattail;
mod chattheme;
mod chattime;
mod chatusage;
mod chatview;
mod chatwidth;
mod chords;
pub mod chrome;
mod clickopen;
mod clipboard;
mod clock;
mod cmdcheck;
mod cmddefs;
mod cmdkeys;
mod cmdmenu;
mod cmdrecents;
mod cmdrow;
mod cmdspan;
pub mod config;
mod confirm;
mod crashlog;
mod ctxlimit;
mod cwd;
mod daemon;
mod daylight;
mod density;
mod detach;
mod diffjob;
mod diffrefine;
mod dispatch;
mod dockicon;
mod dump;
mod ease;
mod editpane;
mod envexpand;
mod envlock;
mod errscan;
mod events;
mod exereplace;
mod faraction;
mod farpane;
mod filedrop;
mod fileindex;
mod findhl;
mod focusmode;
mod fontcmd;
mod fontrotate;
mod fonttick;
mod framegeo;
mod gauges;
mod ghost;
mod git;
mod gitbadge;
mod gitfleet;
#[cfg(test)]
#[path = "glassshot_tests.rs"]
mod glassshot_tests;
mod glide;
mod gradientcmd;
mod gradientlvl;
pub(crate) mod grid;
mod gridrows;
mod gridsel;
mod handler;
mod help;
mod helptable;
mod history;
mod histsearch;
mod hit;
mod host;
pub(crate) mod inputbar;
mod inputbar_render;
mod inputink;
mod inputkeys;
mod ipc;
mod ipc_types;
#[cfg(windows)]
mod ipc_win;
mod keychord;
mod keyentry;
mod keypeek;
mod keyroute;
mod keys;
mod lastout;
mod layout;
mod ledgercli;
mod linkhl;
mod load;
mod md;
mod mentionrange;
mod minstrip;
mod modelfetch;
mod modelpick;
mod modelroute;
mod modernring;
mod motion;
mod navcard;
mod navlog;
mod navlogscroll;
mod navresize;
mod net;
mod notify;
mod oauth;
mod openurl;
mod openview;
mod osappearance;
mod oscontrast;
mod palette;
mod pane;
mod panebtn;
mod panecard;
#[cfg(test)]
#[path = "panecard_budget_tests.rs"]
mod panecard_budget_tests;
mod panecardglow;
mod panedir;
mod panedrag;
mod panefit;
mod panegutter;
mod panehover;
mod panelcard;
mod panelist;
mod panemanage;
mod panes_roster;
mod panescroll;
mod paneview;
mod pastesafe;
mod pathcomplete;
mod pathexpand;
mod pathhl;
mod pointer;
mod poll;
mod procname;
mod quit;
mod readout;
mod reducemotion;
mod relay;
mod render;
mod restart;
mod restartnote;
mod route;
mod runclock;
mod runpane;
mod schemepush;
mod scroll;
mod search;
mod searchall;
mod select;
mod selfupdate;
mod selrun;
mod session;
mod sessionrestore;
mod sessionsave;
mod settingspane;
mod shapecues;
mod shellprobe;
mod smoothlvl;
mod spark;
mod spawn;
mod spotlight;
mod spring;
pub mod stats;
mod statspane;
mod status;
mod suggest;
mod suggestvalues;
mod swarm;
mod swarmpane;
#[cfg(test)]
#[path = "swarmshot_tests.rs"]
mod swarmshot_tests;
mod swatch;
mod termwrite;
mod themefade;
mod themereport;
mod toast;
mod todopane;
mod toggles;
mod tui;
mod unread;
mod update;
mod updatecard;
mod updatefetch;
mod usageledger;
mod viewpane;
mod washfocus;
mod washphase;
mod welcome;
mod wincon;
mod windowtitle;

/// What a bare `crew <args>` invocation wants before any GUI exists.
///
/// Nothing handled `--version`, so it fell through every check and LAUNCHED
/// THE WINDOW — the one thing a person typing `--version` in a terminal
/// definitely does not want, and the natural way to ask which build you are
/// on. `--help` was equally absent while the binary quietly grew half a dozen
/// CLI modes.
///
/// `--help` is honoured only as the FIRST argument, so `crew ask --help`
/// still belongs to the `ask` subcommand rather than being intercepted here.
#[derive(PartialEq, Debug)]
enum CliIntent {
    Version,
    Help,
}

fn cli_intent(args: &[String]) -> Option<CliIntent> {
    if args.iter().any(|a| a == "--version" || a == "-V") {
        return Some(CliIntent::Version);
    }
    match args.first().map(String::as_str) {
        Some("--help" | "-h") => Some(CliIntent::Help),
        _ => None,
    }
}

/// The CLI modes this binary answers, for `--help`.
const CLI_HELP: &str = "\
crew — a multi-pane terminal with agents

usage:
  crew                     open the app (detached from this shell)
  crew --no-detach         open it attached to this shell
  crew ask <agent> <task>  ask a RUNNING crew, print the reply
  crew panes               list a running crew's panes
  crew daemon run          run the resident daemon (crew daemon status to check)
  crew ledger              print what crew did (--limit N)
  crew install-app         add the OS app-menu entry (--remove deletes it)
  crew --list-fonts        print every monospace family the picker offers
  crew --self-update       replace this binary with the latest release
  crew --version           print the version
";

fn main() -> anyhow::Result<()> {
    // First line of the program: a panic before this point would be invisible.
    // A detached crew has stderr on /dev/null and a panic exits through the
    // normal path (so the OS files no crash report) — without this hook the
    // window just disappears and leaves nothing to diagnose.
    crashlog::install();
    // Windows: crew is a GUI-subsystem binary so a Start-menu launch never
    // flashes a console, which also means a terminal launch starts with no
    // stdio at all. Reattach to the launching shell's console before anything
    // below prints. Skipped for the detached GUI child — it is detached from
    // that terminal on purpose, and it never prints. Handles the shell already
    // set up (pipes, redirects, the broker's JSON-line stdio) are left alone;
    // see `wincon`.
    if !detach::is_detached_child() {
        wincon::attach_to_parent();
    }
    // Answered before anything else: these must never reach the GUI launch.
    let args: Vec<String> = std::env::args().skip(1).collect();
    match cli_intent(&args) {
        Some(CliIntent::Version) => {
            println!("crew {}", appregister::VERSION);
            return Ok(());
        }
        Some(CliIntent::Help) => {
            print!("{CLI_HELP}");
            return Ok(());
        }
        None => {}
    }
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
    // Inter-pane ask client subcommands (`crew ask …` / `crew panes`): connect
    // to a RUNNING crew's IPC socket, print the reply, exit. Placed before the
    // detach re-launch — a client must never spawn a GUI, it talks to the one
    // already up. All routing lives in askclient so main stays a thin launcher.
    if let Some(code) = askclient::dispatch_cli() {
        std::process::exit(code);
    }
    // `crew daemon …` — the resident. Placed with the other client subcommands,
    // before the detach re-launch and any GUI init: `daemon run` is a headless
    // foreground process that must never open a window, and `daemon status`
    // must answer on a box with no display at all.
    if let Some(code) = daemon::cli::dispatch_cli() {
        std::process::exit(code);
    }
    // `crew ledger` — read back the action ledger. A trail nobody can read is not an audit
    // trail, so the reader is a first-class subcommand rather than "cat this JSONL".
    if let Some(code) = ledgercli::dispatch_cli() {
        std::process::exit(code);
    }
    // `crew install-app` — create/refresh the OS app-menu entry (Spotlight /
    // Start menu / .desktop); `--remove` deletes it. Also run automatically
    // by install.sh and silently on GUI startup.
    if std::env::args().skip(1).any(|a| a == "install-app") {
        return if std::env::args().skip(1).any(|a| a == "--remove") {
            appregister::remove_current()
        } else {
            appregister::register_current(true)
        };
    }
    // Detached launch is the default: re-launch in a new session (detached from
    // this terminal) and exit the parent, so closing the launching shell doesn't
    // SIGHUP the GUI. `--no-detach` / `--foreground` keeps crew attached. The
    // re-launched child sets CREW_DETACHED, so it falls through to the GUI.
    if detach::should_detach() && !detach::is_detached_child() {
        return detach::relaunch_detached();
    }
    // Only the GUI path forks/reads a login shell to seed PATH and provider
    // keys — CLI modes above (broker/self-update/list-fonts/install-app/detach
    // re-exec) return before this line, so they never pay for a shell they
    // don't use. One probe covers both PATH (`cmdcheck::effective_path`) and
    // provider-key discovery (`shellprobe::provider_now`/`openrouter_key`).
    shellprobe::init_probe();
    // An in-place update leaves the superseded binary beside the new one:
    // Windows cannot delete an image while it is running, so the update that
    // installed *this* build could not clean up after itself. Best effort, and
    // silent — a failure just means it is still in use.
    if let Ok(exe) = std::env::current_exe() {
        exereplace::sweep_leftovers(&exe);
    }
    handler::run()
}

#[cfg(test)]
mod cli_intent_tests {
    use super::{cli_intent, CliIntent};

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    /// The bug this exists for: `--version` used to match nothing and fall
    /// through to launching the window.
    #[test]
    fn version_is_answered_not_launched() {
        assert_eq!(cli_intent(&args(&["--version"])), Some(CliIntent::Version));
        assert_eq!(cli_intent(&args(&["-V"])), Some(CliIntent::Version));
        // Anywhere in the line — nobody should have to remember the position.
        assert_eq!(
            cli_intent(&args(&["--no-detach", "--version"])),
            Some(CliIntent::Version)
        );
    }

    #[test]
    fn help_is_answered_when_it_leads() {
        assert_eq!(cli_intent(&args(&["--help"])), Some(CliIntent::Help));
        assert_eq!(cli_intent(&args(&["-h"])), Some(CliIntent::Help));
    }

    /// …but a subcommand owns its own `--help`. `crew ask --help` is a
    /// question about `ask`, and intercepting it here would answer the wrong
    /// one.
    #[test]
    fn a_subcommand_keeps_its_own_help() {
        assert_eq!(cli_intent(&args(&["ask", "--help"])), None);
        assert_eq!(cli_intent(&args(&["panes", "-h"])), None);
    }

    /// Everything else falls through to the launcher, as before.
    #[test]
    fn ordinary_invocations_still_launch() {
        assert_eq!(cli_intent(&args(&[])), None);
        assert_eq!(cli_intent(&args(&["--no-detach"])), None);
        assert_eq!(cli_intent(&args(&["install-app"])), None);
    }
}
