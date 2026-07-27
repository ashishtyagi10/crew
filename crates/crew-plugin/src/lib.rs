mod broker;
pub mod credentials;
mod echo;
mod host;
pub mod mcp;
mod orchestrator;
mod protocol;
pub use broker::{
    active_provider, broker_constructs, direct_by_name, explain_output, known_adapters,
    no_provider_advice, parse_routing, run_broker_stdio, skills_list, suggest_command,
    suggest_far_command, Adapter, Broker, CliAdapter, DirectProvider, Envelope, Hop, HopKind,
    Normalize, Provider, Registry, Routing, RunStats, Skill, ToolRunner, DIRECT,
};
pub use echo::respond;
pub use host::Plugin;
pub use orchestrator::plan;
pub use protocol::{AgentInfo, PluginCommand, PluginEvent};
