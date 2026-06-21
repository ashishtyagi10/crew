mod app;
mod chat;
mod chatlayout;
pub mod chrome;
pub mod config;
mod gauges;
mod handler;
mod layout;
mod pane;
mod render;
mod session;
mod settingspane;
mod spawn;
pub mod stats;
mod statspane;

fn main() -> anyhow::Result<()> {
    handler::run()
}
