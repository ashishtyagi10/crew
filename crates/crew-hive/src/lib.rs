//! # crew-hive
//!
//! The orchestration substrate for running many agents toward a shared goal.
//!
//! ## Modules
//! - [`graph`] — typed task-graph: [`TaskGraph`], [`TaskSpec`], [`TaskId`], dependency edges
//! - [`bus`] — non-blocking broadcast event bus: [`EventBus`], [`HiveEvent`]
//! - [`board`] — shared blackboard for task results and artifacts: [`Blackboard`]
//! - [`telemetry`] — live fleet telemetry aggregation: [`Fleet`], [`FleetTotals`]
//! - [`agent`] — agent trait + context + stub implementations: [`Agent`], [`AgentFactory`]
//! - [`sched`] — tokio-based DAG executor with bounded concurrency: [`Scheduler`]
//!
//! ## Quick start
//! ```rust,no_run
//! use crew_hive::{TaskGraph, TaskSpec, TaskId, AgentKind, ModelTier,
//!                 Blackboard, EventBus, Scheduler, StubAgent, AgentFactory};
//! ```

pub mod agent;
pub mod apiagent;
pub mod batch;
pub mod board;
pub mod bus;
pub mod govern;
pub mod graph;
pub mod planner;
pub mod provider;
pub mod sched;
pub mod telemetry;
pub mod view;

// Graph
pub use graph::{AgentKind, GraphError, ModelTier, TaskGraph, TaskId, TaskSpec, TaskState};

// Bus
pub use bus::{AgentId, EventBus, HiveEvent};

// Board
pub use board::{Blackboard, BlackboardSnapshot, TaskResult};

// Telemetry
pub use telemetry::{AgentTelemetry, Fleet, FleetTotals};

// Agent
pub use agent::{Agent, AgentContext, AgentFactory, StubAgent};

// ApiAgent
pub use apiagent::ApiAgent;

// Scheduler
pub use sched::{RunOutcome, Scheduler};

// Provider
pub use provider::{
    AnthropicProvider, Completion, CompletionRequest, MockProvider, Provider, ProviderError,
};

// Planner
pub use planner::{LlmPlanner, PlanError, Planner, StubPlanner};

// Batch
pub use batch::{batch_graph, Job};

// Govern
pub use govern::{budget_governor, Budget};
