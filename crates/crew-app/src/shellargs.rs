//! The arguments a pane's shell starts with.
//!
//! On Unix, `-l`: a login shell sources the user's profile, which is where
//! PATH, the provider keys and their prompt (oh-my-zsh, say) live — like
//! Ghostty or Terminal. No Windows shell has that flag, and PowerShell and
//! cmd.exe both *reject* `-l`, so passing it there failed every spawn; they
//! read their profile unconditionally anyway. A PowerShell pane gets the
//! oh-my-posh start-up instead (see [`crate::posh`]).

/// What `shell` is started with in a pane.
#[cfg(unix)]
pub(crate) fn for_pane(_shell: &str) -> Vec<String> {
    vec!["-l".to_string()]
}

/// What `shell` is started with in a pane.
#[cfg(windows)]
pub(crate) fn for_pane(shell: &str) -> Vec<String> {
    if crate::posh::is_powershell(shell) {
        crate::posh::args()
    } else {
        Vec::new()
    }
}
