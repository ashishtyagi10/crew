//! Read-only language intelligence over the Language Server Protocol.
//!
//! Two consumers, one client: the swarm's agents get `lsp:*` tools (hover,
//! definition, references, diagnostics) so they reason with real symbols
//! rather than grep hits, and the file viewer shows a server's diagnostics
//! beside the code. Nothing here EDITS anything — the viewer's design docs
//! forbid code editing in the pane, and a diagnostic overlay is not editing.
//!
//! The wire is JSON-RPC 2.0 over the child's stdio with `Content-Length`
//! framing ([`framing`]); a reader thread splits what arrives into responses,
//! notifications and server-to-client requests ([`demux`]); [`Client`] owns
//! the process, the handshake and the open documents; [`ops`] are the typed
//! helpers; [`servers`] says which binary serves which language; [`root`]
//! finds the project a file belongs to; [`running`] is the process-global
//! list of live servers the `/lsp` status card reads.
//!
//! The MCP client in crew-plugin is the precedent — but MCP is line-framed
//! and only ever hears replies, while a language server frames by length and
//! talks first (`textDocument/publishDiagnostics`), so the reader here demuxes
//! rather than drops.
pub mod client;
pub mod demux;
pub mod docs;
pub mod framing;
pub mod ops;
pub mod root;
pub mod running;
pub mod servers;
pub mod types;
pub mod uri;

pub use client::Client;
pub use demux::Notification;
pub use types::{Diagnostic, Location, Position, Range, Severity};
