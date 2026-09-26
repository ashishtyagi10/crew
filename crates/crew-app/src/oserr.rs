//! OS error text as crew says things.
//!
//! An `io::Error` displays as `No such file or directory (os error 2)`, and
//! every `format!("…: {e}")` carried that onto the bar and into an error
//! toast: `failed to spawn shell: No such file or directory (os error 2)` —
//! a capital mid-sentence and an errno nobody at the keyboard can use. The
//! one place a status passes through ([`crate::app::CrewApp::set_status_level`])
//! runs it through here.

/// `msg` with every ` (os error N)` removed, and the OS sentence it closed
/// lower-cased at its first letter (`No such file…` → `no such file…`) —
/// unless that word is an acronym (`EOF`), which keeps its case.
pub(crate) fn plain(msg: String) -> String {
    const TAG: &str = " (os error ";
    let mut out = msg;
    while let Some(at) = out.find(TAG) {
        let Some(close) = out[at..].find(')').map(|i| at + i) else {
            break;
        };
        let digits = &out[at + TAG.len()..close];
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            break;
        }
        out.replace_range(at..=close, "");
        let start = out[..at].rfind(": ").map_or(0, |i| i + 2);
        let mut word = out[start..].chars();
        if let (Some(a), Some(b)) = (word.next(), word.next()) {
            if a.is_uppercase() && b.is_lowercase() {
                let lower: String = a.to_lowercase().collect();
                out.replace_range(start..start + a.len_utf8(), &lower);
            }
        }
    }
    out
}

#[cfg(test)]
#[path = "oserr_tests.rs"]
mod tests;
