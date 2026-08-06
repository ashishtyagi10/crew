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
mod boxdraw;
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
mod cmdmenu;
pub mod config;
mod crashlog;
mod ctxlimit;
mod cwd;
mod detach;
mod dispatch;
mod dockicon;
mod dump;
mod ease;
mod editpane;
mod envexpand;
mod envlock;
mod events;
mod faraction;
mod farpane;
mod filedrop;
mod fileindex;
mod findhl;
mod fontcmd;
mod fontrotate;
mod fonttick;
mod framegeo;
mod gauges;
mod ghost;
mod git;
#[cfg(test)]
#[path = "glassshot_tests.rs"]
mod glassshot_tests;
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
mod ipc;
mod ipc_types;
mod keyentry;
mod keys;
mod layout;
mod linkhl;
mod load;
mod md;
mod mentionrange;
mod minstrip;
mod modelfetch;
mod modelpick;
mod modelroute;
mod motion;
mod navcard;
mod navlog;
mod net;
mod notify;
mod oauth;
mod openurl;
mod openview;
mod palette;
mod pane;
mod panecard;
mod panecardglow;
mod panefit;
mod panelcard;
mod panelist;
mod panemanage;
mod panes_roster;
mod paneview;
mod pathcomplete;
mod pathexpand;
mod poll;
mod procname;
mod quit;
mod readout;
mod relay;
mod render;
mod restart;
mod restartnote;
mod route;
mod runpane;
mod scroll;
mod search;
mod searchall;
mod select;
mod selfupdate;
mod session;
mod sessionrestore;
mod sessionsave;
mod settingspane;
mod shellprobe;
mod spark;
mod spawn;
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
mod termwrite;
mod toast;
mod toggles;
mod tui;
mod update;
mod updatecard;
mod updatefetch;
mod usageledger;
mod viewpane;
mod welcome;
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
