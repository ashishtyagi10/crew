//! The planner's error type. Split out of `planner/mod.rs` so that file —
//! the prompt, the parser and the security forcing — stays inside the line
//! cap as the parser learned to mint a persona per task.
use crate::graph::GraphError;
use crate::provider::ProviderError;

#[derive(Debug)]
pub enum PlanError {
    Provider(ProviderError),
    Parse(String),
    Graph(GraphError),
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanError::Provider(e) => write!(f, "provider error: {e}"),
            PlanError::Parse(s) => write!(f, "parse error: {s}"),
            PlanError::Graph(e) => write!(f, "graph error: {e}"),
        }
    }
}

impl std::error::Error for PlanError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PlanError::Provider(e) => Some(e),
            PlanError::Graph(e) => Some(e),
            PlanError::Parse(_) => None,
        }
    }
}

impl From<ProviderError> for PlanError {
    fn from(e: ProviderError) -> Self {
        PlanError::Provider(e)
    }
}

impl From<GraphError> for PlanError {
    fn from(e: GraphError) -> Self {
        PlanError::Graph(e)
    }
}
