//! crew-hive: the orchestration substrate for running many agents toward a
//! task. Foundations: a typed task-graph (`graph`), a non-blocking event bus
//! (`bus`), and a fleet telemetry model (`telemetry`).

pub mod bus;
pub mod graph;
pub mod telemetry;
