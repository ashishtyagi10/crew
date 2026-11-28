//! What the nested `sys:explore` agent runs on (`sysexplore`): its read-only
//! tool surface, the meter that prices its model calls, and the bill the
//! asking turn collects.
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use crew_hive::harness::Events;
use crew_hive::{HiveEvent, Spent, ToolSpec, Tools};

/// The only tools the nested agent may use: everything that only looks.
pub(super) const ALLOWED: [&str; 6] = ["read_file", "list_dir", "grep", "glob", "outline", "git"];

/// What nested turns have cost since the engine last asked (`Tools::take_spent`).
#[derive(Default)]
pub(super) struct Bill(Mutex<Spent>);

impl Bill {
    pub(super) fn take(&self) -> Spent {
        std::mem::take(&mut *self.0.lock().unwrap_or_else(|e| e.into_inner()))
    }
    pub(super) fn add(&self, s: Spent) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) += s;
    }
}

/// Adds up what the nested turn's model calls cost; shows nothing.
#[derive(Default)]
pub(super) struct Meter(Mutex<Spent>);

impl Meter {
    pub(super) fn total(&self) -> Spent {
        *self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl Events for Meter {
    fn publish(&self, ev: HiveEvent) {
        let add = match ev {
            HiveEvent::TokenDelta { input, output, .. } => Spent {
                input: input.into(),
                output: output.into(),
                micros_usd: 0,
            },
            HiveEvent::CostDelta { micros_usd, .. } => Spent {
                micros_usd,
                ..Spent::default()
            },
            _ => return,
        };
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) += add;
    }
}

/// The nested agent's whole tool surface: [`ALLOWED`] `sys` tools, run
/// through `systools::call` (so `CREW_SYS_MODE` and the workspace fence still
/// apply), with a count of what it looked at.
#[derive(Default)]
pub(super) struct Reader {
    files: AtomicU32,
    searches: AtomicU32,
}

impl Reader {
    pub(super) fn counts(&self) -> String {
        format!(
            "explored: {} files read, {} searches",
            self.files.load(Ordering::Relaxed),
            self.searches.load(Ordering::Relaxed)
        )
    }
}

impl Tools for Reader {
    fn hint(&self) -> String {
        super::toolcall::hint_for(&self.catalog())
    }
    fn specs(&self) -> Vec<ToolSpec> {
        self.catalog()
            .into_iter()
            .map(|t| ToolSpec {
                server: t.server,
                tool: t.name,
                description: t.description,
                input_schema: t.input_schema,
            })
            .collect()
    }
    fn repeatable(&self, _: &str, _: &str) -> bool {
        true
    }
    fn call(&self, server: &str, tool: &str, args: &str) -> Result<String, String> {
        if server != "sys" || !ALLOWED.contains(&tool) {
            return Err(format!(
                "{server}:{tool} is not available here \u{2014} this researcher can only read: {}",
                ALLOWED.join(", ")
            ));
        }
        let n = if tool == "read_file" {
            &self.files
        } else {
            &self.searches
        };
        n.fetch_add(1, Ordering::Relaxed);
        super::systools::call(tool, args)
    }
}

impl Reader {
    fn catalog(&self) -> Vec<crate::mcp::McpTool> {
        let mut all = super::systools::tools();
        all.retain(|t| ALLOWED.contains(&t.name.as_str()));
        all
    }
}
