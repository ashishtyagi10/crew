//! `crew daemon install` — the whole consent for a background service.
//!
//! Kept in its own file for the reason the doc comment below gives: nothing else in crew may
//! call it, and a reader looking for what installs a login service should find one small file
//! rather than a function buried in a router.

/// Whether the opt-in login service is installed for this user.
pub(super) fn service_state() -> &'static str {
    let Some(home) = dirs::home_dir() else {
        return "unknown (no home directory)";
    };
    match std::env::current_exe()
        .ok()
        .and_then(|exe| super::service::unit_for_host(&exe))
    {
        Some(unit) if super::service::is_installed(&home, &unit) => "installed",
        Some(_) => "not installed (crew daemon install)",
        None => "unsupported on this platform",
    }
}

/// `crew daemon install` / `--remove`. Nothing else in crew may call this: a background service
/// the user did not ask for turns a bad release into a login loop instead of an `/update`.
pub(super) fn install(remove: bool) -> i32 {
    let Some(home) = dirs::home_dir() else {
        println!("cannot find your home directory");
        return 1;
    };
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            println!("cannot locate the crew binary: {e}");
            return 1;
        }
    };
    let Some(unit) = super::service::unit_for_host(&exe) else {
        println!("crew has no service integration for this platform yet");
        return 1;
    };
    if remove {
        if let Err(e) = super::service::run_step(&home, &unit.deactivate) {
            println!("could not deactivate the service: {e}");
        }
        return match super::service::remove_unit(&home, &unit) {
            Ok(true) => {
                println!("the crew daemon will no longer start at login");
                0
            }
            Ok(false) => {
                println!("the crew daemon was not installed");
                0
            }
            Err(e) => {
                println!("could not remove the service file: {e}");
                1
            }
        };
    }
    match super::service::write_unit(&home, &unit) {
        Ok(path) => {
            println!("wrote {}", path.display());
            match super::service::run_step(&home, &unit.activate) {
                Ok(()) => println!("the crew daemon will start at login (and is starting now)"),
                Err(e) => println!(
                    "wrote the service file but could not activate it: {e}\n\
                     activate it yourself with: {}",
                    unit.activate.join(" ")
                ),
            }
            0
        }
        Err(e) => {
            println!("could not write the service file: {e}");
            1
        }
    }
}
