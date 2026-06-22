mod app;
mod boxdraw;
mod chat;
mod chatlayout;
mod chords;
pub mod chrome;
mod clipboard;
mod clock;
mod cmdmenu;
pub mod config;
mod cwd;
mod gauges;
mod handler;
mod help;
mod history;
mod hit;
mod host;
pub(crate) mod inputbar;
mod inputkeys;
mod keys;
mod layout;
mod matrix;
mod net;
mod pane;
mod paneview;
mod render;
mod scroll;
mod search;
mod session;
mod settingspane;
mod spawn;
pub mod stats;
mod statspane;
mod suggest;
mod tui;
mod welcome;
mod windowtitle;

fn main() -> anyhow::Result<()> {
    handler::run()
}
