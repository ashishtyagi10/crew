//! Persisting the input-bar command history across sessions.
use std::path::PathBuf;

/// Keep at most this many recent lines on disk.
const MAX: usize = 1000;

fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("crew").join("history"))
}

/// The last `MAX` lines, newline-joined (oldest first).
fn serialize(history: &[String]) -> String {
    let start = history.len().saturating_sub(MAX);
    history[start..].join("\n")
}

/// Parse stored history: non-empty lines, oldest first.
fn deserialize(s: &str) -> Vec<String> {
    s.lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

/// Load the persisted command history (empty if none / unreadable).
pub fn load() -> Vec<String> {
    let Some(p) = path() else {
        return Vec::new();
    };
    std::fs::read_to_string(&p)
        .map(|s| deserialize(&s))
        .unwrap_or_default()
}

/// Persist the command history (capped to the most recent `MAX` lines).
pub fn save(history: &[String]) {
    let Some(p) = path() else {
        return;
    };
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&p, serialize(history));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_filters_blanks() {
        let h = vec!["ls".to_string(), "cargo test".to_string()];
        assert_eq!(deserialize(&serialize(&h)), h);
        assert_eq!(
            deserialize("a\n\n b \n"),
            vec!["a".to_string(), " b ".to_string()]
        );
    }

    #[test]
    fn serialize_caps_to_max() {
        let h: Vec<String> = (0..MAX + 50).map(|i| i.to_string()).collect();
        let out = deserialize(&serialize(&h));
        assert_eq!(out.len(), MAX);
        assert_eq!(out.first().unwrap(), "50"); // oldest 50 dropped
    }
}
