mod broker;
mod echo;
mod host;
pub mod mcp;
mod orchestrator;
mod protocol;
pub use broker::{
    explain_output, known_adapters, parse_routing, run_broker_stdio, suggest_command,
    suggest_far_command, Adapter, Broker, CliAdapter, Envelope, Hop, HopKind, Normalize, Registry,
    Routing, RunStats, ToolRunner,
};
pub use echo::respond;
pub use host::Plugin;
pub use orchestrator::plan;
pub use protocol::{AgentInfo, PluginCommand, PluginEvent};
