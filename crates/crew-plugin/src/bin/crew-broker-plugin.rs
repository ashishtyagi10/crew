//! The `/crew` broker as a standalone plugin binary: discovers the installed
//! coding agents (claude, codex, opencode) and relays a task between them over
//! the JSON plugin protocol, streaming one event per hop. The same loop is also
//! reachable as `crew --broker-plugin`, so a `/crew` pane works without a
//! separate install. All logic lives in `crew_plugin::run_broker_stdio`.
fn main() -> anyhow::Result<()> {
    crew_plugin::run_broker_stdio()
}
