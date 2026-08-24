mod broker;
/// The action gate: who may fire an irreversible tool, and what silence means.
pub mod approval {
    pub use crate::broker::approval::*;
}
/// The append-only record of what crew did.
pub mod ledger {
    pub use crate::broker::ledger::*;
}
/// Tool tiering: what a tool can do to the world, and whether it can be undone.
pub mod tier {
    pub use crate::broker::tier::*;
}
pub mod credentials;
mod echo;
mod host;
pub mod mcp;
mod orchestrator;
mod protocol;
pub use broker::{
    active_provider, broker_constructs, construct_summary, direct_by_name, expand_alias,
    explain_output, known_adapters, no_provider_advice, parse_routing, run_broker_stdio,
    skills_list, suggest_command, suggest_far_command, Adapter, Broker, CliAdapter, DirectProvider,
    Envelope, Hop, HopKind, Normalize, Provider, Registry, Routing, RunStats, Skill, ToolRunner,
    DIRECT,
};
pub use echo::respond;
pub use host::Plugin;
pub use orchestrator::plan;
pub use protocol::{AgentInfo, PluginCommand, PluginEvent};
