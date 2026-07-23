//! # crew-hive
//!
//! The orchestration substrate ("the Hive") for running many agents toward a
//! shared goal: decompose a goal into a task-graph, execute it over a bounded
//! pool of agents, merge results, and stream live telemetry for a swarm view.
//! Headless and UI-independent — `crew-app` drives it; nothing here touches the
//! GPU or a terminal.
//!
//! ## Modules
//! Core:
//! - [`graph`] — typed task-graph (DAG): [`TaskGraph`], [`TaskSpec`], [`TaskId`], [`AgentKind`], [`ModelTier`]
//! - [`bus`] — non-blocking broadcast event bus: [`EventBus`], [`HiveEvent`], [`AgentId`]
//! - [`board`] — shared blackboard for task results + artifacts: [`Blackboard`], [`TaskResult`]
//! - [`telemetry`] — live fleet telemetry aggregation: [`Fleet`], [`FleetTotals`]
//! - [`agent`] — agent trait + context + stub: [`Agent`], [`AgentFactory`], [`StubAgent`]
//! - [`sched`] — tokio DAG executor (bounded concurrency, cascade-cancel, cooperative cancellation): [`Scheduler`]
//!
//! Brain & workers (bring-your-own-LLM):
//! - [`provider`] — LLM provider abstraction: [`Provider`], [`MockProvider`], [`AnthropicProvider`]
//! - [`planner`] — goal → task-graph: [`Planner`], [`StubPlanner`], [`LlmPlanner`]
//! - [`apiagent`] — native LLM agent (futures, no PTY): [`ApiAgent`]
//!
//! Scale & control:
//! - [`batch`] — flat parallel-job graph: [`batch_graph`], [`Job`]
//! - [`govern`] — cost ceiling: [`Budget`], [`budget_governor`]
//!
//! Remote / sidecar (out-of-process & external engines):
//! - [`wire`] — JSON wire protocol + [`Transport`]: [`RemoteTask`], [`RemoteReply`]
//! - [`worker`] — [`LoopbackTransport`] + [`serve_stdio`] worker codec
//! - [`remoteagent`] — agent dispatched over a [`Transport`]: [`RemoteAgent`]
//!
//! ## Quick start
//! ```rust,no_run
//! use crew_hive::{TaskGraph, TaskSpec, TaskId, AgentKind, ModelTier,
//!                 Blackboard, EventBus, Scheduler, StubAgent, AgentFactory};
//! ```

pub mod agent;
pub mod agentname;
pub mod apiagent;
pub mod batch;
pub mod board;
pub mod bus;
pub mod govern;
pub mod graph;
pub mod planner;
pub mod pricing;
pub mod provider;
pub mod remoteagent;
pub mod sched;
pub mod telemetry;
pub mod wire;
pub mod worker;

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

// AgentName
pub use agentname::{role_clamp, slug, slug_or};

// ApiAgent
pub use apiagent::{ApiAgent, ApiFactory};

// Scheduler
pub use sched::{RunOutcome, Scheduler};

// Provider
pub use provider::{
    AnthropicProvider, ChunkFn, Completion, CompletionRequest, MockProvider, OpenRouterProvider,
    Provider, ProviderError,
};

// Planner
pub use planner::{LlmPlanner, PlanError, Planner, StubPlanner};

// Batch
pub use batch::{batch_graph, Job};

// Govern
pub use govern::{budget_governor, Budget};

// Wire
pub use wire::{DepResult, RemoteReply, RemoteTask, Transport, TransportError};

// Worker
pub use worker::{serve_stdio, LoopbackTransport};

// RemoteAgent
pub use remoteagent::{RemoteAgent, RemoteFactory};
