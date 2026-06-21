mod app;
mod chat;
mod chatlayout;
mod handler;
mod layout;
mod pane;
mod session;

fn main() -> anyhow::Result<()> {
    handler::run()
}
