mod broker;
mod echo;
mod host;
mod orchestrator;
mod protocol;
pub use broker::{
    known_adapters, parse_routing, run_broker_stdio, Adapter, Broker, CliAdapter, Envelope, Hop,
    HopKind, Normalize, Registry, Routing,
};
pub use echo::respond;
pub use host::Plugin;
pub use orchestrator::plan;
pub use protocol::{PluginCommand, PluginEvent};
