//! The `lsp` tool server: language servers, started per project root and
//! language on first use and kept for the session, answering the four
//! read-only questions an agent asks about code — what is this symbol, where
//! is it defined, where is it used, what is wrong with this file.
//!
//! Shaped like [`crate::mcp::McpHost`]: lazy clients, a tool list the session
//! merges into its catalog, one `call` the dispatch chain routes to. Unlike
//! MCP the tools are crew's own, so their tier is declared here as READ
//! (`broker/tier.rs`) rather than defaulting to "ask".
mod tools;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crew_lsp::servers::{self, Server};
use crew_lsp::{Client, Diagnostic};

use crate::mcp::{EventSink, McpTool, StatusSink};
pub use tools::Args;

/// Per-request deadline. A language server that has not answered in this
/// long is indexing something large; the agent is told so and can retry.
pub const TIMEOUT: Duration = Duration::from_secs(15);
/// How long `diagnostics` waits for a publish on a file already open — the
/// server has usually said its piece, but a fresh save may be in flight.
const SETTLE: Duration = Duration::from_millis(1500);

#[derive(Default)]
pub struct LspHost {
    servers: BTreeMap<String, Server>,
    clients: BTreeMap<(String, PathBuf), Client>,
    /// The latest `publishDiagnostics` per URI, from every notification
    /// drained while answering anything.
    diags: BTreeMap<String, Vec<Diagnostic>>,
    /// Where a start that failed goes (`lsp <cmd>: <why>`, for the LOG) and
    /// where one that succeeded goes (a `Loaded` line in the pane's tool
    /// block). A language server used to start with no sink, no status and
    /// no log line: rust-analyzer's first index can take a minute, and the
    /// only sign anything was happening was the tool call's clock.
    sink: Option<StatusSink>,
    events: Option<EventSink>,
    /// Failures already noted, keyed `lang:root` — a missing binary is
    /// re-tried by every call, and would otherwise hit the LOG every call.
    noted: std::collections::BTreeSet<String>,
}

impl LspHost {
    /// A host over an explicit server table (tests).
    pub fn new(servers: BTreeMap<String, Server>) -> Self {
        Self {
            servers,
            ..Self::default()
        }
    }

    /// The built-in table with `lsp.json` merged over it; empty under the
    /// mock provider so scripted broker tests stay deterministic.
    pub fn from_config() -> Self {
        if std::env::var("CREW_BROKER_MOCK_REPLY").is_ok() {
            return Self::default();
        }
        Self::new(servers::load())
    }

    /// Whether any language is served at all.
    pub fn is_empty(&self) -> bool {
        self.servers.is_empty()
    }

    /// Route start failures to `sink` and successful starts to `events`.
    pub fn set_sinks(&mut self, sink: StatusSink, events: EventSink) {
        self.sink = Some(sink);
        self.events = Some(events);
    }

    /// Start the server for `(lang, root)`, telling the sinks how it went.
    fn start(&mut self, lang: &str, root: &Path) -> Result<Client, String> {
        let key = format!("{lang}:{}", root.display());
        let started = self.server_for(lang).and_then(|(server, bin)| {
            let mut c = Client::spawn(&bin.to_string_lossy(), &server.args, lang, root)?;
            c.initialize(TIMEOUT)?;
            Ok((server.command, c))
        });
        match started {
            Ok((command, c)) => {
                self.noted.remove(&key);
                if let Some(events) = &self.events {
                    events(loaded_event(lang, &command, root));
                }
                Ok(c)
            }
            Err(e) => {
                let cmd = self.servers.get(lang).map_or(lang, |s| s.command.as_str());
                if self.noted.insert(key) {
                    if let Some(sink) = &self.sink {
                        sink(true, &format!("lsp {cmd}: {e}"));
                    }
                }
                Err(e)
            }
        }
    }

    /// The four tools, when there is a server table to answer them.
    pub fn tools(&self) -> Vec<McpTool> {
        if self.is_empty() {
            return Vec::new();
        }
        tools::descriptors()
    }

    /// The server for `lang`, or why there is none — "not installed" names
    /// the binary, since that is the thing to fix.
    fn server_for(&self, lang: &str) -> Result<(Server, PathBuf), String> {
        let server = self
            .servers
            .get(lang)
            .ok_or_else(|| format!("no language server configured for {lang}"))?;
        let bin = servers::which(&server.command).ok_or_else(|| {
            format!(
                "{} is not installed (it serves {lang}) \u{2014} install it, or point \
                 ~/.config/crew/lsp.json at another server",
                server.command
            )
        })?;
        Ok((server.clone(), bin))
    }

    /// The running client for `(lang, root)`, started on first use.
    fn client(&mut self, lang: &str, root: &Path) -> Result<&mut Client, String> {
        let key = (lang.to_string(), root.to_path_buf());
        if !self.clients.contains_key(&key) {
            let c = self.start(lang, root)?;
            self.clients.insert(key.clone(), c);
        }
        Ok(self.clients.get_mut(&key).expect("just inserted"))
    }

    /// Record a notification if it is a diagnostics publish; the URI it was
    /// about, if so.
    fn absorb(
        diags: &mut BTreeMap<String, Vec<Diagnostic>>,
        n: crew_lsp::Notification,
    ) -> Option<String> {
        if n.method != "textDocument/publishDiagnostics" {
            return None;
        }
        let (uri, list) = Diagnostic::parse_publish(&n.params)?;
        diags.insert(uri.clone(), list);
        Some(uri)
    }

    /// Wait up to `timeout` for a publish about `uri`, recording every
    /// publish seen on the way. `true` if one arrived.
    fn wait_publish(&mut self, key: &(String, PathBuf), uri: &str, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let Some(c) = self.clients.get(key) else {
            return false;
        };
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let Some(n) = c.wait_notification(left) else {
                return false;
            };
            if Self::absorb(&mut self.diags, n).as_deref() == Some(uri) {
                return true;
            }
        }
    }

    /// Run one `lsp` tool. Every failure is a sentence for the agent; a
    /// failed request also drops the client so the next call starts fresh.
    pub fn call(&mut self, tool: &str, args: &str) -> Result<String, String> {
        let a = Args::parse(args)?;
        let path = tools::absolute(&a.file);
        let lang = servers::lang_of(&path)
            .ok_or_else(|| format!("no language server for {} files", tools::ext_of(&path)))?;
        let root = crew_lsp::root::for_file(&path);
        let key = (lang.to_string(), root.clone());
        let uri = crew_lsp::uri::from_path(&path);
        let fresh = {
            let c = self.client(lang, &root)?;
            if c.is_open(&uri) {
                false
            } else {
                let text = std::fs::read_to_string(&path)
                    .map_err(|e| format!("{}: {e}", path.display()))?;
                c.did_open(&uri, lang, &text)?;
                true
            }
        };
        let (line, col) = (a.line.saturating_sub(1), a.col.saturating_sub(1));
        let out = match tool {
            "diagnostics" => {
                let wait = if fresh { TIMEOUT } else { SETTLE };
                self.wait_publish(&key, &uri, wait);
                let list = self.diags.get(&uri).cloned().unwrap_or_default();
                Ok(tools::diagnostics_text(&root, &path, &list))
            }
            _ => {
                let c = self.clients.get_mut(&key).expect("opened above");
                match tool {
                    "hover" => c
                        .hover(&uri, line, col, TIMEOUT)
                        .map(|h| tools::hover_text(&root, &path, &a, h)),
                    "definition" | "references" => {
                        let r = if tool == "definition" {
                            c.definition(&uri, line, col, TIMEOUT)
                        } else {
                            c.references(&uri, line, col, TIMEOUT)
                        };
                        r.map(|locs| tools::locations_text(tool, &root, &path, &a, &locs))
                    }
                    other => Err(crew_hive::tools::near::unknown(
                        "lsp tool",
                        other,
                        &tools::NAMES,
                        "",
                    )),
                }
            }
        };
        if out.is_err() {
            self.clients.remove(&key);
        }
        out
    }

    /// The `/lsp` status card's server rows: language, command, installed?
    pub fn status_rows(&self) -> Vec<(String, String, bool)> {
        self.servers
            .iter()
            .map(|(lang, s)| {
                (
                    lang.clone(),
                    s.command.clone(),
                    servers::which(&s.command).is_some(),
                )
            })
            .collect()
    }
}

/// The `Loaded` line for a server that answered `initialize`: the command
/// (what a person would install or configure), the language it serves and
/// the project it indexes — `rust-analyzer · rust · crew`.
pub(crate) fn loaded_event(lang: &str, command: &str, root: &Path) -> crew_hive::HiveEvent {
    let project = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string());
    crew_hive::HiveEvent::Loaded {
        agent: String::new(),
        kind: "lsp".into(),
        name: command.to_string(),
        detail: format!("{lang} \u{b7} {project}"),
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
