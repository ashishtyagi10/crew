//! The provider every model call of a capped task goes through: ask first when
//! the task is over its cap, count the cost when the call lands.
//!
//! WHY a provider and not a hook in the harness loops: the relay's turns and
//! the swarm's workers each build their own loop, but both end at a
//! `Provider`, and a call is the one thing that spends. Wrapping it checks
//! before every dial (the first, a repair re-ask, each round's follow-up, the
//! last-word call) without touching either loop, and a task that is stopped at
//! the cap sees an ordinary stop (`Ended::Stopped`), settles and gets its
//! receipt as for `/stop`.
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crew_hive::{ChunkFn, Completion, CompletionRequest, Provider, ProviderError};

use super::spendcap::{self, TaskSpend};

type Call = Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>>;

/// `inner` under the running task's cap, or `inner` itself when this thread
/// runs no capped task (cap off, a test, a command). `model` prices a reply
/// that came without a cost.
pub(crate) fn wrap(inner: Arc<dyn Provider>, model: &str) -> Arc<dyn Provider> {
    match spendcap::current() {
        Some(task) => Arc::new(Capped {
            inner,
            model: model.to_string(),
            task,
        }),
        None => inner,
    }
}

/// [`wrap`] for a swarm's provider, whose first question comes at the $1 the
/// old silent governor (`Budget::DEFAULT_MICROS_USD`) killed it at: a wide
/// swarm spends fastest, so it earns the earliest ask, and a yes goes on by
/// the configured step as for any task.
pub(crate) fn wrap_swarm(inner: Arc<dyn Provider>, model: &str) -> Arc<dyn Provider> {
    if let Some(task) = spendcap::current() {
        task.first_ask_at(crew_hive::Budget::DEFAULT_MICROS_USD);
    }
    wrap(inner, model)
}

/// What a reply cost: its own figure, else the price list's estimate, else 0
/// for a model with no price (the cap never fires on what it cannot price).
pub(crate) fn cost(model: &str, c: &Completion) -> u64 {
    match c.cost_microusd {
        0 => crew_hive::pricing::estimate(model, c).unwrap_or(0),
        paid => paid,
    }
}

/// A provider that holds each call at the task's cap.
pub(crate) struct Capped {
    pub(crate) inner: Arc<dyn Provider>,
    pub(crate) model: String,
    pub(crate) task: Arc<TaskSpend>,
}

impl Capped {
    fn call(&self, req: CompletionRequest, on_chunk: Option<ChunkFn>) -> Call {
        let (inner, task) = (Arc::clone(&self.inner), Arc::clone(&self.task));
        let model = self.model.clone();
        Box::pin(async move {
            task.before_call().await.map_err(ProviderError::Http)?;
            let c = match on_chunk {
                Some(f) => inner.complete_streaming(req, f).await?,
                None => inner.complete(req).await?,
            };
            task.add(cost(&model, &c));
            Ok(c)
        })
    }
}

impl Provider for Capped {
    fn supports_tools(&self) -> bool {
        self.inner.supports_tools()
    }

    fn complete(&self, req: CompletionRequest) -> Call {
        self.call(req, None)
    }

    fn warm(&self) {
        self.inner.warm();
    }

    fn complete_streaming(&self, req: CompletionRequest, on_chunk: ChunkFn) -> Call {
        self.call(req, Some(on_chunk))
    }
}
