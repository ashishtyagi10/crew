mod app;
mod boxdraw;
mod chat;
mod chatlayout;
pub mod chrome;
mod clock;
mod cmdmenu;
pub mod config;
mod gauges;
mod handler;
mod help;
pub(crate) mod inputbar;
mod keys;
mod layout;
mod matrix;
mod pane;
mod render;
mod session;
mod settingspane;
mod spawn;
pub mod stats;
mod statspane;
mod suggest;
mod tui;
mod welcome;

fn main() -> anyhow::Result<()> {
    handler::run()
}
