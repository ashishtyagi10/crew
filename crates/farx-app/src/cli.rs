use clap::Parser;

#[derive(Parser)]
#[command(
    name = "farx",
    version,
    about = "Next-generation cross-platform file manager"
)]
pub struct Cli {
    /// Update farx to the latest release
    #[arg(long)]
    pub update: bool,

    /// Check if a newer version is available
    #[arg(long)]
    pub check_update: bool,

    /// Print version
    #[arg(long)]
    pub keydebug: bool,
}

/// Handle `--update`: download and install the latest version.
pub fn run_update() -> anyhow::Result<()> {
    println!("farx — checking for updates...");
    match farx_core::update::perform_update()? {
        self_update::Status::UpToDate(v) => {
            println!("Already up to date (v{v}).");
        }
        self_update::Status::Updated(v) => {
            println!("Updated to v{v}! Restart farx to use the new version.");
        }
    }
    Ok(())
}

/// Handle `--check-update`: print whether an update exists.
pub fn run_check_update() {
    use farx_core::update;
    update::print_version();
    let rx = update::check_and_auto_update_async();
    match rx.recv() {
        Ok(update::UpdateStatus::Updated(v)) => {
            println!("Updated to v{v}! Restart farx to use the new version.");
        }
        Ok(update::UpdateStatus::Available(v)) => {
            println!("New version available: v{v}");
            println!("Run `farx --update` to install it.");
        }
        Ok(update::UpdateStatus::UpToDate) => {
            println!("You are on the latest version.");
        }
        Ok(update::UpdateStatus::Failed(e)) => {
            eprintln!("Could not check for updates: {e}");
        }
        Err(_) => {
            eprintln!("Update check did not complete.");
        }
    }
}
