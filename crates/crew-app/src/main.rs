mod app;
mod chat;
mod chatlayout;
mod handler;
mod layout;
mod pane;
mod session;
mod spawn;

fn main() -> anyhow::Result<()> {
    handler::run()
}
