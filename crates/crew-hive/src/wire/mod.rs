//! Wire protocol: JSON-line types exchanged between scheduler and remote workers.
use serde::{Deserialize, Serialize};
use std::fmt;
use std::future::Future;
use std::pin::Pin;

#[cfg(test)]
mod tests;

/// One dependency's result sent alongside a remote task.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DepResult {
    pub task: u64,
    pub output: String,
    pub success: bool,
}

/// Dispatch envelope sent to a remote worker.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RemoteTask {
    pub agent: u64,
    pub task: u64,
    pub prompt: String,
    pub model: String,
    pub deps: Vec<DepResult>,
}

/// Reply envelope received from a remote worker.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RemoteReply {
    pub task: u64,
    pub output: String,
    pub success: bool,
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Errors that may arise during transport dispatch.
#[derive(Debug)]
pub enum TransportError {
    Send(String),
    Recv(String),
    Decode(String),
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportError::Send(s) => write!(f, "transport send error: {s}"),
            TransportError::Recv(s) => write!(f, "transport recv error: {s}"),
            TransportError::Decode(s) => write!(f, "transport decode error: {s}"),
        }
    }
}

impl std::error::Error for TransportError {}

/// Object-safe transport: dispatches a `RemoteTask` and returns a `RemoteReply`.
/// Uses a boxed future so `Arc<dyn Transport>` works without async-trait.
pub trait Transport: Send + Sync {
    fn dispatch(
        &self,
        task: RemoteTask,
    ) -> Pin<Box<dyn Future<Output = Result<RemoteReply, TransportError>> + Send>>;
}
