//! The pane, the history and the helpers the rewind tests share.
use super::*;
use crate::chatlayout::Message;
use crew_plugin::{Plugin, PluginEvent};

/// A pane whose broker is a file: everything the pane sends lands in it.
pub(crate) fn pane() -> (ChatPane, std::path::PathBuf) {
    static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let file = std::env::temp_dir().join(format!(
        "crew-rewind-sent-{}-{}",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _ = std::fs::remove_file(&file);
    let script = format!("cat >'{}'", file.display());
    let plugin = Plugin::spawn("sh", &["-c".to_string(), script]).unwrap();
    let mut p = ChatPane::new(plugin, "crew".into());
    p.connected = true;
    (p, file)
}

/// What the pane has sent so far, waiting (bounded) for `n` lines.
pub(crate) fn sent(file: &std::path::Path, n: usize) -> Vec<String> {
    for _ in 0..200 {
        let lines: Vec<String> = std::fs::read_to_string(file)
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect();
        if lines.len() >= n {
            return lines;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    Vec::new()
}

pub(crate) fn say(p: &mut ChatPane, sender: &str, ts: &str, text: &str) {
    p.push_capped(Message {
        sender: sender.into(),
        text: text.into(),
        ts: ts.into(),
        meta: String::new(),
        usage: None,
        expanded: false,
    });
}

pub(crate) fn started(p: &mut ChatPane, id: u64, label: &str) {
    p.rewind_hears(&PluginEvent::Task {
        id,
        label: label.into(),
        running: true,
    });
    p.absorb_task(id, true);
    p.absorb_task(id, false);
}

/// Two tasks and one slash command answered inline, with replies between.
pub(crate) fn history() -> (ChatPane, std::path::PathBuf) {
    let (mut p, file) = pane();
    say(&mut p, "user", "100", "fix the parser");
    started(&mut p, 1, "fix the parser");
    say(&mut p, "coder", "101", "done, parser fixed");
    say(&mut p, "user", "200", "/diff");
    say(&mut p, "agent smith", "201", "no changes");
    say(&mut p, "user", "300", "now add tests");
    started(&mut p, 2, "now add tests");
    say(&mut p, "coder", "301", "tests added");
    (p, file)
}

pub(crate) fn answer(
    task: u64,
    files: u32,
    outside: bool,
    applied: bool,
    error: &str,
) -> PluginEvent {
    PluginEvent::Rewind {
        task,
        files,
        outside,
        applied,
        error: error.into(),
    }
}

/// What the card draws, as `has` reads it: blank cells are not cells, so
/// both sides are compared with their spaces taken out.
pub(crate) struct Drawn(pub String);

impl Drawn {
    pub(crate) fn contains(&self, needle: &str) -> bool {
        self.0.contains(&needle.replace(' ', ""))
    }
}

impl std::fmt::Display for Drawn {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

pub(crate) fn card_text(p: &ChatPane) -> Drawn {
    let cells = p.popup(100).expect("a card").cells;
    Drawn(cells.iter().map(|c| c.c).filter(|c| *c != ' ').collect())
}

pub(crate) fn key(p: &mut ChatPane, k: ChatInput) {
    p.on_input(k, &std::env::temp_dir());
}
