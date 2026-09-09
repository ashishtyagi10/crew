//! The diagnostics behind the viewer's margin markers: a language server
//! started for the file's project, told about the file, listened to until
//! it has spoken, and shut down — all on a worker thread, because a server
//! indexing a workspace takes seconds and every pane in the grid is frozen
//! for as long as anything blocks the winit thread (the blame read has the
//! same shape, see [`super::blamejob`]).
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

use crew_lsp::{servers, Client, Diagnostic};

use super::detect::Format;
use super::{LoadState, ViewPane};

/// How long the server may take to start, and then to first speak.
const START: Duration = Duration::from_secs(20);
/// After the first publish, how long to wait for a fuller one — rust-analyzer's
/// `cargo check` pass follows its own native one by a second or two.
const GRACE: Duration = Duration::from_secs(2);

fn read(path: &Path, lang: &str) -> Result<Vec<Diagnostic>, String> {
    let server = servers::load()
        .remove(lang)
        .ok_or_else(|| format!("no server for {lang}"))?;
    let bin = servers::which(&server.command)
        .ok_or_else(|| format!("{} is not installed", server.command))?;
    let root = crew_lsp::root::for_file(path);
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut c = Client::spawn(&bin.to_string_lossy(), &server.args, lang, &root)?;
    c.initialize(START)?;
    let uri = crew_lsp::uri::from_path(path);
    c.did_open(&uri, lang, &text)?;
    let mut list = c
        .wait_diagnostics(&uri, START)
        .ok_or("the server never reported")?;
    while let Some(later) = c.wait_diagnostics(&uri, GRACE) {
        list = later;
    }
    c.shutdown();
    Ok(list)
}

/// What the viewer knows from a language server about its file.
#[derive(Default)]
pub(crate) enum Lsp {
    /// Not asked yet — the file has not landed.
    #[default]
    Off,
    /// Asked and declined: not a code file, no server for its language, the
    /// server not installed, or `lsp = false`. Stays until a re-read.
    Skipped,
    Loading {
        rx: Receiver<Result<Vec<Diagnostic>, String>>,
        /// The command, for the legend's "starting…".
        server: String,
    },
    On(Vec<Diagnostic>),
    Failed(String),
}

/// `2 errors · 1 warning`; `no diagnostics` for an empty list. Information
/// and hints are not counted: the margin does not mark them either.
pub(crate) fn summary(diags: &[Diagnostic]) -> String {
    let errors = diags.iter().filter(|d| d.is_error()).count();
    let warnings = diags
        .iter()
        .filter(|d| d.severity == crew_lsp::Severity::Warning)
        .count();
    let word = |n: usize, one: &str| format!("{n} {one}{}", if n == 1 { "" } else { "s" });
    match (errors, warnings) {
        (0, 0) => "no diagnostics".into(),
        (e, 0) => word(e, "error"),
        (0, w) => word(w, "warning"),
        (e, w) => format!("{} \u{b7} {}", word(e, "error"), word(w, "warning")),
    }
}

impl Lsp {
    pub(crate) fn start(path: PathBuf, lang: &'static str, server: String) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(read(&path, lang));
        });
        Lsp::Loading { rx, server }
    }

    /// The diagnostics to draw, once there are any.
    pub(crate) fn diags(&self) -> Option<&[Diagnostic]> {
        match self {
            Lsp::On(d) if !d.is_empty() => Some(d),
            _ => None,
        }
    }

    /// The legend's word on it, while there is one worth a legend. A
    /// failure is said too: a margin that never appears cannot explain why.
    pub(crate) fn status(&self) -> Option<String> {
        match self {
            Lsp::Loading { server, .. } => Some(format!("lsp: {server} starting\u{2026}")),
            Lsp::On(d) => Some(summary(d)),
            Lsp::Failed(why) => Some(format!("lsp: {why}")),
            Lsp::Off | Lsp::Skipped => None,
        }
    }

    /// Drain the worker. `true` on the tick it settled.
    pub(crate) fn poll(&mut self) -> bool {
        let Lsp::Loading { rx, .. } = self else {
            return false;
        };
        *self = match rx.try_recv() {
            Ok(Ok(list)) => Lsp::On(list),
            Ok(Err(why)) => Lsp::Failed(why),
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => Lsp::Failed("the server job stopped".into()),
        };
        true
    }
}

impl ViewPane {
    /// Ask once the file has landed — a code file, a server on PATH for its
    /// language, the switch on — then drain the answer. `true` on the tick
    /// anything changed, which is what tells the poll to redraw.
    pub(crate) fn poll_lsp(&mut self) -> bool {
        if let Lsp::Off = self.lsp {
            let ready = matches!(
                &self.state,
                LoadState::Ready {
                    format: Format::Code { .. },
                    ..
                }
            );
            if !ready {
                return false;
            }
            let lang = servers::lang_of(&self.path);
            let server = lang
                .and_then(|l| servers::load().remove(l))
                .filter(|s| servers::which(&s.command).is_some());
            // Never under test: the switch is a process global that a config
            // apply in a parallel test flips on, and a `.rs` fixture would
            // then start a real rust-analyzer on the temp dir.
            let on = crate::lspon::on() && !cfg!(test);
            self.lsp = match (on, lang, server) {
                (true, Some(lang), Some(s)) => Lsp::start(self.path.clone(), lang, s.command),
                _ => Lsp::Skipped,
            };
            return matches!(self.lsp, Lsp::Loading { .. });
        }
        let changed = self.lsp.poll();
        if changed {
            // The margin is part of the layout: the text must re-wrap
            // narrower by the column, so the cache is rebuilt, not decorated.
            self.cache.replace(None);
        }
        changed
    }
}

#[cfg(test)]
#[path = "lspjob_tests.rs"]
mod tests;
