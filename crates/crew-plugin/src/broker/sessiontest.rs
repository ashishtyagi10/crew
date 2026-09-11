//! Test-only constructors for `SessionTools`, and the crowded fixture the retrieval tests
//! share. Moved out of `session.rs` so that file could take the tool decider without
//! growing past its debt — nothing here changed in the move except `for_test`'s picker,
//! which is the scorer-only one: a live chooser in a test surface would be a network call.
//! A child of `session`, so the struct's private fields and `new` are in reach.
use std::sync::{Arc, Mutex};

use super::toolmemo::Picker;
use super::SessionTools;
use crate::broker::approval::{Gate, Requester};
use crate::broker::integration::Integration;

impl SessionTools {
    /// A runner with a gate of its own, for tests that do not care which
    /// gate — the session-shared one is not reachable from here.
    pub(super) fn for_test(mcp: Arc<Mutex<crate::mcp::McpHost>>, sys: bool) -> Self {
        Self::new(
            mcp,
            Arc::new(Mutex::new(crate::lsp::LspHost::default())),
            sys,
            Arc::new(Mutex::new(Gate::new())),
            Arc::new(Picker::off()),
        )
    }

    /// [`Self::for_test`] with a language-server table handed in.
    pub(super) fn with_lsp(self, lsp: crate::lsp::LspHost) -> Self {
        Self {
            lsp: Arc::new(Mutex::new(lsp)),
            ..self
        }
    }

    /// [`Self::for_test`] with integrations handed in rather than read from disk. The
    /// discovery path is tested in `integration::tests`; wiring it through `CREW_PROJECT_DIR`
    /// here would put a process-global env var in a suite that runs in parallel — the exact
    /// flake the `sys` field's comment in `session.rs` records.
    pub(super) fn with_integrations(sys: bool, integrations: Vec<Integration>) -> Self {
        Self {
            integrations,
            ..Self::for_test(Arc::new(Mutex::new(crate::mcp::McpHost::default())), sys)
        }
    }

    /// The same runner answering to somebody who is NOT at the keyboard. Unused until a channel
    /// exists to carry the question; it is the reason the gate is wired now rather than later.
    pub(super) fn for_requester(
        mcp: Arc<Mutex<crate::mcp::McpHost>>,
        sys: bool,
        requester: Requester,
    ) -> Self {
        Self {
            requester,
            ledger: None,
            ..Self::for_test(mcp, sys)
        }
    }

    /// The same runner with a tool decider handed in — an injected chooser, shared with
    /// whoever else needs to read what it decided.
    pub(super) fn with_picker(self, picker: Arc<Picker>) -> Self {
        Self { picker, ..self }
    }
}

/// A manifest with more tools than the budget and none of crew's own: the shape in which
/// the native path used to hide tools without saying so, and without the door — and now
/// the shape in which the model, not the scorer, decides what is shown.
pub(super) fn crowded(sys: bool, picker: Arc<Picker>) -> SessionTools {
    let tools: Vec<String> = (0..crate::broker::toolpick::BUDGET + 8)
        .map(|i| {
            format!(
                r#"{{"name": "thing{i}", "description": "does an unrelated thing",
                    "path": "/t/{i}", "tier": "read"}}"#
            )
        })
        .collect();
    let manifest = format!(
        r#"{{"name": "noise", "base_url": "https://api.example.com",
            "auth": {{"kind": "bearer", "env": "NOISE_TEST_TOKEN"}},
            "tools": [{}]}}"#,
        tools.join(",")
    );
    let int = crate::broker::integration::parse(&manifest).expect("a valid manifest");
    SessionTools::with_integrations(sys, vec![int]).with_picker(picker)
}

/// [`crowded`] as the surface a swarm factory takes — for tests outside `session`.
pub(crate) fn crowded_tools(sys: bool, picker: Arc<Picker>) -> Arc<dyn crew_hive::tools::Tools> {
    Arc::new(crowded(sys, picker))
}
