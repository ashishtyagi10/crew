mod broker;
pub mod credentials;
mod echo;
mod host;
pub mod mcp;
mod orchestrator;
mod protocol;
pub use broker::{
    active_provider, explain_output, known_adapters, parse_routing, run_broker_stdio, skills_list,
    suggest_command, suggest_far_command, Adapter, Broker, CliAdapter, Envelope, Hop, HopKind,
    Normalize, Provider, Registry, Routing, RunStats, Skill, ToolRunner,
};
pub use echo::respond;
pub use host::Plugin;
pub use orchestrator::plan;
pub use protocol::{AgentInfo, PluginCommand, PluginEvent};
