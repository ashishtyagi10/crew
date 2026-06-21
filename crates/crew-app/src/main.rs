mod app;
mod boxdraw;
mod chat;
mod chatlayout;
pub mod chrome;
pub mod config;
mod gauges;
mod handler;
pub(crate) mod inputbar;
mod keys;
mod layout;
mod pane;
mod render;
mod session;
mod settingspane;
mod spawn;
pub mod stats;
mod statspane;
mod tui;
mod welcome;

fn main() -> anyhow::Result<()> {
    handler::run()
}
