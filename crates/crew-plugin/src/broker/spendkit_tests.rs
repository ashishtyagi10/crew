//! Shared fixtures for the spend cap's tests: a provider at a known price, a
//! task whose asker answers at once, and a block-on.
use crate::broker::spendcap::{Ask, Say, TaskSpend};
use crate::broker::spendcapwrap::Capped;
use crew_hive::{Completion, CompletionRequest, Provider, ProviderError};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub(crate) type Reply =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<Completion, ProviderError>> + Send>>;

/// A provider that answers every call with `micros` of cost.
pub(crate) struct Priced {
    micros: u64,
    calls: AtomicUsize,
}

impl Priced {
    pub(crate) fn of(micros: u64) -> Arc<Self> {
        Arc::new(Self {
            micros,
            calls: AtomicUsize::new(0),
        })
    }
    pub(crate) fn made(&self) -> usize {
        self.calls.load(Ordering::Relaxed)
    }
}

impl Provider for Priced {
    fn complete(&self, _req: CompletionRequest) -> Reply {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let c = Completion {
            text: "ok".into(),
            cost_microusd: self.micros,
            ..Default::default()
        };
        Box::pin(async move { Ok(c) })
    }
}

/// What a test's pane saw and said.
pub(crate) struct Pane {
    pub(crate) asked: Mutex<Vec<String>>,
    pub(crate) said: Mutex<Vec<String>>,
    pub(crate) cancel: Arc<AtomicBool>,
}

/// A task capped at `cap` micro-USD whose asker answers `answer` at once
/// (`None`: nobody answers).
pub(crate) fn task(cap: u64, answer: Option<bool>) -> (Arc<TaskSpend>, Arc<Pane>) {
    let pane = Arc::new(Pane {
        asked: Mutex::default(),
        said: Mutex::default(),
        cancel: Arc::default(),
    });
    let p = Arc::clone(&pane);
    let ask: Ask = Arc::new(move |id, q| {
        p.asked.lock().unwrap().push(q.to_string());
        if let Some(a) = answer {
            crate::broker::approval::deliver_answer(id, a);
        }
        true
    });
    let p = Arc::clone(&pane);
    let say: Say = Arc::new(move |l| p.said.lock().unwrap().push(l.to_string()));
    let t = TaskSpend::new(cap, Arc::clone(&pane.cancel), ask, say).unwrap();
    (t.waiting(Duration::from_millis(150)), pane)
}

pub(crate) fn run<T>(f: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(f)
}

pub(crate) fn capped(inner: &Arc<Priced>, task: &Arc<TaskSpend>) -> Capped {
    Capped {
        inner: Arc::clone(inner) as Arc<dyn Provider>,
        model: "scripted".into(),
        task: Arc::clone(task),
    }
}

pub(crate) async fn call(p: &Capped) -> Result<Completion, ProviderError> {
    p.complete(CompletionRequest::default()).await
}

/// Tokens and no price.
pub(crate) struct Tokens;

impl Provider for Tokens {
    fn complete(&self, _req: CompletionRequest) -> Reply {
        let c = Completion {
            text: "ok".into(),
            input_tokens: 5_000_000,
            output_tokens: 5_000_000,
            ..Default::default()
        };
        Box::pin(async move { Ok(c) })
    }
}
