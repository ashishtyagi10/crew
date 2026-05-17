use anyhow::Result;
use clap::Parser;

mod cli;
mod install;
mod keydebug;
mod tui;

use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    // --keydebug: interactive key event debugger
    if args.keydebug {
        keydebug::run_key_debug();
        return Ok(());
    }

    // --update: download and install the latest version, then exit
    if args.update {
        return cli::run_update();
    }

    // --check-update: just print whether an update exists
    if args.check_update {
        cli::run_check_update();
        return Ok(());
    }

    tui::run_tui().await
}
