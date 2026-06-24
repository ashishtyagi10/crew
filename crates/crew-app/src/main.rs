mod about;
mod app;
mod boxdraw;
mod chat;
mod chatlayout;
mod chatspawn;
mod chords;
pub mod chrome;
mod clickopen;
mod clipboard;
mod clock;
mod cmdmenu;
pub mod config;
mod cwd;
mod dispatch;
mod dump;
mod editpane;
mod envexpand;
mod events;
mod farpane;
mod findhl;
mod fontcmd;
mod gauges;
mod git;
mod gridrows;
mod handler;
mod help;
mod history;
mod histsearch;
mod hit;
mod host;
pub(crate) mod inputbar;
mod inputkeys;
mod keys;
mod layout;
mod linkhl;
mod load;
mod matrix;
mod net;
mod openurl;
mod pane;
mod panecard;
mod panelist;
mod panemanage;
mod paneview;
mod pathcomplete;
mod pathexpand;
mod poll;
mod quit;
mod reload;
mod render;
mod runpane;
mod scroll;
mod search;
mod session;
mod settingspane;
mod spawn;
pub mod stats;
mod statspane;
mod status;
mod suggest;
mod termwrite;
mod toggles;
mod tui;
mod welcome;
mod windowtitle;

fn main() -> anyhow::Result<()> {
    // When the `/crew` pane spawns this binary as its multi-agent broker (a
    // re-exec of `crew` with this flag), run the JSON-line broker loop and exit
    // before any GUI initialization. This means `/crew` works wherever `crew`
    // is installed, with no separate plugin binary to ship.
    if std::env::args().skip(1).any(|a| a == "--broker-plugin") {
        return crew_plugin::run_broker_stdio();
    }
    handler::run()
}
