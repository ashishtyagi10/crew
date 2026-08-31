//! A minimal MCP (Model Context Protocol) client: stdio transport,
//! line-delimited JSON-RPC 2.0. Servers are declared in `mcp.json` (the same
//! `mcpServers` schema other coding tools use), connected lazily, and exposed
//! to the `/crew` relay as callable tools.
mod client;
pub(crate) mod config;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub use client::McpClient;
pub use config::ServerConfig;

/// Where connection-lifecycle notes go (`error`, `message`) — the broker
/// wires this to a `PluginEvent::Status` emit so the host's activity LOG
/// shows servers connecting, ready, or failing. `None` (tests, standalone
/// use) keeps the host silent.
pub type StatusSink = Arc<dyn Fn(bool, &str) + Send + Sync>;

/// One callable tool on a connected server.
///
/// `Eq` is gone because `input_schema` is a `serde_json::Value` (floats).
/// `PartialEq` is what tests compare with; nothing hashed or ordered these.
#[derive(Debug, Clone, PartialEq)]
pub struct McpTool {
    pub server: String,
    pub name: String,
    /// The server's FULL description, newlines and all.
    ///
    /// It used to be truncated to its first line at the point of parsing, for
    /// a text hint that then clipped it again to 100 characters. Native
    /// tool-use wants the whole thing — servers put the usage notes that
    /// disambiguate two similar tools in the second paragraph — so the
    /// shortening moved to the hint, which is the only place that needs it.
    pub description: String,
    /// JSON Schema for the arguments, straight from `tools/list`.
    /// `{"type":"object"}` when the server declared none.
    pub input_schema: serde_json::Value,
}

/// All configured MCP servers: lazy connections plus a per-server tool cache,
/// so listing tools costs one `tools/list` per server per session.
#[derive(Default)]
pub struct McpHost {
    servers: BTreeMap<String, ServerConfig>,
    clients: BTreeMap<String, McpClient>,
    cache: BTreeMap<String, Vec<McpTool>>,
    /// Track `mcp.json` on every use (only hosts built by [`Self::from_config`]),
    /// so config edits land without a restart. Explicit maps (tests) stay pinned.
    auto: bool,
    /// Lifecycle notes destination; `None` = silent.
    sink: Option<StatusSink>,
    /// Note keys already sent for the current connection generation. A dead
    /// server is retried on every turn (`client()` caches only successes), so
    /// without this the same connect failure would hit the LOG per turn.
    /// Cleared per server when its config changes and wholesale on `/reload`,
    /// so a reconnect announces itself again.
    noted: BTreeSet<String>,
}

impl McpHost {
    /// A host over an explicit server map (used by tests).
    pub fn new(servers: BTreeMap<String, ServerConfig>) -> Self {
        Self {
            servers,
            ..Self::default()
        }
    }

    /// A host over the merged `mcp.json` config. Empty under
    /// `CREW_BROKER_MOCK_REPLY` so mock-driven broker tests stay deterministic
    /// on machines that have real servers configured.
    pub fn from_config() -> Self {
        if std::env::var("CREW_BROKER_MOCK_REPLY").is_ok() {
            return Self::default();
        }
        Self {
            auto: true,
            ..Self::new(config::load())
        }
    }

    /// Whether any server is configured at all (after a config sync, so the
    /// first server added to `mcp.json` enables tools without a restart).
    pub fn is_empty(&mut self) -> bool {
        self.sync();
        self.servers.is_empty()
    }

    /// Route lifecycle notes to `sink` (the broker's `Status` event emit).
    pub fn set_sink(&mut self, sink: StatusSink) {
        self.sink = Some(sink);
    }

    /// Send one note through the sink, at most once per `key` per connection
    /// generation (see `noted`). An empty key always sends.
    fn note_once(&mut self, key: String, error: bool, msg: &str) {
        if !key.is_empty() && !self.noted.insert(key) {
            return;
        }
        if let Some(sink) = &self.sink {
            sink(error, msg);
        }
    }

    /// Re-read `mcp.json` when this host tracks it; a no-op for pinned maps.
    fn sync(&mut self) {
        if self.auto {
            self.sync_to(config::load());
        }
    }

    /// Adopt `fresh` as the server map: servers that were removed or whose
    /// config changed lose their client (killing the child) and cached tools;
    /// unchanged servers keep their live connection and cache.
    pub(crate) fn sync_to(&mut self, fresh: BTreeMap<String, ServerConfig>) {
        if fresh == self.servers {
            return;
        }
        let stale: Vec<String> = self
            .servers
            .keys()
            .filter(|name| fresh.get(*name) != self.servers.get(*name))
            .cloned()
            .collect();
        for name in stale {
            self.clients.remove(&name);
            self.cache.remove(&name);
            // New config = new connection generation: announce again.
            self.noted
                .retain(|k| k != &name && !k.starts_with(&format!("{name}:")));
        }
        self.servers = fresh;
    }

    /// Force a full refresh for `/reload`: re-read `mcp.json` (when this host
    /// tracks it), then drop every client and the whole tool cache so the next
    /// use reconnects and re-lists. Returns the configured server names.
    pub fn reload(&mut self) -> Vec<String> {
        self.sync();
        self.clients.clear();
        self.cache.clear();
        self.noted.clear();
        self.servers.keys().cloned().collect()
    }

    /// The connected client for `server`, connecting on first use.
    fn client(&mut self, server: &str) -> Result<&mut McpClient, String> {
        let Some(cfg) = self.servers.get(server) else {
            let known: Vec<&str> = self.servers.keys().map(|s| s.as_str()).collect();
            return Err(format!(
                "unknown MCP server \u{201c}{server}\u{201d} \u{2014} configured: {}",
                if known.is_empty() {
                    "(none)".into()
                } else {
                    known.join(", ")
                }
            ));
        };
        if !self.clients.contains_key(server) {
            let cfg = cfg.clone();
            self.note_once(
                format!("{server}:connect"),
                false,
                &format!("mcp \u{21c4} {server} connecting\u{2026}"),
            );
            let c = match McpClient::connect(&cfg) {
                Ok(c) => c,
                Err(e) => {
                    self.note_once(
                        format!("{server}:fail"),
                        true,
                        &format!("mcp {server}: {e}"),
                    );
                    return Err(e);
                }
            };
            self.clients.insert(server.to_string(), c);
        }
        Ok(self.clients.get_mut(server).expect("just inserted"))
    }

    /// One server's tools, fetched once and cached for the session.
    fn fetch(&mut self, server: &str) -> Result<Vec<McpTool>, String> {
        if let Some(t) = self.cache.get(server) {
            return Ok(t.clone());
        }
        let list = match self.client(server)?.tools() {
            Ok(list) => list,
            Err(e) => {
                // Connected but the tool listing failed — worth its own note
                // (the connect keys are already spent by `client()`).
                self.note_once(
                    format!("{server}:fail"),
                    true,
                    &format!("mcp {server}: {e}"),
                );
                return Err(e);
            }
        };
        self.note_once(
            String::new(),
            false,
            &format!("mcp {server} connected \u{b7} {} tool(s)", list.len()),
        );
        let tools: Vec<McpTool> = list
            .into_iter()
            .map(|(name, description, input_schema)| McpTool {
                server: server.to_string(),
                name,
                description,
                input_schema,
            })
            .collect();
        self.cache.insert(server.to_string(), tools.clone());
        Ok(tools)
    }

    /// Every tool on every configured server (cached after the first fetch).
    /// A server that fails to connect or list contributes nothing here — the
    /// failure is visible in [`McpHost::report`].
    pub fn tools(&mut self) -> Vec<McpTool> {
        self.sync();
        let names: Vec<String> = self.servers.keys().cloned().collect();
        names
            .iter()
            .flat_map(|s| self.fetch(s).unwrap_or_default())
            .collect()
    }

    /// Call `tool` on `server` with JSON `args` (empty = `{}`). A failed call
    /// drops the connection so the next call reconnects fresh.
    pub fn call(&mut self, server: &str, tool: &str, args: &str) -> Result<String, String> {
        self.sync();
        let args = args.trim();
        let value: serde_json::Value = if args.is_empty() {
            serde_json::json!({})
        } else {
            serde_json::from_str(args)
                .map_err(|e| format!("tool arguments are not valid JSON: {e}"))?
        };
        let res = self.client(server)?.call(tool, value);
        if res.is_err() {
            self.clients.remove(server);
            // A drop starts a new connection generation: free the server's
            // note keys so the reconnect announces itself (once) too.
            self.noted
                .retain(|k| k != server && !k.starts_with(&format!("{server}:")));
            // The chat surface already carries the call's own error; this
            // notes the connection consequence. Unkeyed: each drop is a real,
            // separate lifecycle event.
            self.note_once(
                String::new(),
                true,
                &format!("mcp {server} connection dropped \u{2014} will reconnect"),
            );
        }
        res
    }

    /// The `/mcp` listing: each server with its tools, or its failure.
    pub fn report(&mut self) -> String {
        if self.is_empty() {
            return "No MCP servers configured. Declare them in \
                    ~/.config/crew/mcp.json or ./.crew/mcp.json as \
                    {\"mcpServers\": {\"name\": {\"command\": \"\u{2026}\", \
                    \"args\": [\u{2026}]}}} \u{2014} agents can then call their \
                    tools mid-relay."
                .into();
        }
        let names: Vec<String> = self.servers.keys().cloned().collect();
        let mut lines = Vec::new();
        for server in names {
            match self.fetch(&server) {
                Ok(list) => {
                    let tools: Vec<&str> = list.iter().map(|t| t.name.as_str()).collect();
                    lines.push(format!(
                        "\u{25aa} {server} \u{2014} {} tool(s): {}",
                        tools.len(),
                        tools.join(", ")
                    ));
                }
                Err(e) => lines.push(format!("\u{25aa} {server} \u{2014} error: {e}")),
            }
        }
        lines.join("\n")
    }
}
