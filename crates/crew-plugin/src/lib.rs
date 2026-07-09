mod broker;
mod echo;
mod host;
pub mod mcp;
mod orchestrator;
mod protocol;
pub use broker::{
    known_adapters, parse_routing, run_broker_stdio, suggest_command, Adapter, Broker, CliAdapter,
    Envelope, Hop, HopKind, Normalize, Registry, Routing, RunStats, ToolRunner,
};
pub use echo::respond;
pub use host::Plugin;
pub use orchestrator::plan;
pub use protocol::{AgentInfo, PluginCommand, PluginEvent};
