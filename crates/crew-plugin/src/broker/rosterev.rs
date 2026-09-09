//! The one place a `Roster` event is built: the agents, who serves the
//! API-backed seats, and the sign-ins on offer. Eight call sites used to
//! spell the event by hand; a field added to it would have been a field
//! seven of them forgot.
use crate::{AgentInfo, PluginEvent};

/// The roster event for `agents`, stamped with the resolved provider and
/// the machine's sign-in rows (from the per-process probe cache — never a
/// fresh CLI spawn on this path).
pub(crate) fn roster(agents: Vec<AgentInfo>) -> PluginEvent {
    PluginEvent::Roster {
        agents,
        provider: super::discover::resolved_provider().map(|p| p.name().to_string()),
        signins: super::loginrows::options(&super::logincmd::rows_cached()),
    }
}
