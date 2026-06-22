mod app;
mod boxdraw;
mod chat;
mod chatlayout;
pub mod chrome;
mod clipboard;
mod clock;
mod cmdmenu;
pub mod config;
mod gauges;
mod handler;
mod help;
mod host;
pub(crate) mod inputbar;
mod keys;
mod layout;
mod matrix;
mod pane;
mod paneview;
mod render;
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
