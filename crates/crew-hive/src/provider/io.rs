//! One always-running runtime for provider HTTP, and a warm connection pool
//! on it.
//!
//! Measured (2026-09-27, DashScope international): opening the connection —
//! TCP and TLS — cost ~0.5 s, the request itself ~0.37 s. Every call paid the
//! open, because every call built its own client: the router's call, then the
//! answer's, a second handshake to the same host a second later.
//!
//! A shared client cannot simply be handed around: a pooled connection is
//! driven by a task on the runtime that OPENED it, and crew runs provider
//! calls on short-lived and current-thread runtimes (a blocking one-shot, an
//! adapter's own runtime that only runs inside its `block_on`). A request
//! handed a connection whose runtime is not being polled waits for the read
//! timeout. So the work itself moves here: [`run`] spawns the request onto
//! this runtime, which never stops, and the caller awaits the result from
//! whatever runtime it is on. [`client`] is only ever used inside [`run`].
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Mutex, OnceLock};
use std::task::{Context, Poll};
use std::time::Duration;

use super::ProviderError;

fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("crew-provider-io")
            .enable_all()
            .build()
            .expect("provider io runtime")
    })
}

/// The pooled client for `timeout` — one per distinct timeout, shared by every
/// provider instance. Use it only inside [`run`].
pub(crate) fn client(timeout: Duration) -> reqwest::Client {
    static POOL: OnceLock<Mutex<HashMap<u128, reqwest::Client>>> = OnceLock::new();
    let mut m = POOL
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    m.entry(timeout.as_millis())
        .or_insert_with(|| super::http_client(timeout))
        .clone()
}

/// Run `fut` on the provider runtime and await it from here. Dropping the
/// returned future aborts the request — a caller's timeout still cancels it,
/// and a cancelled stream stops calling its chunk callback.
pub(crate) fn run<F, T>(fut: F) -> Pin<Box<dyn Future<Output = Result<T, ProviderError>> + Send>>
where
    F: Future<Output = Result<T, ProviderError>> + Send + 'static,
    T: Send + 'static,
{
    let handle = runtime().spawn(fut);
    Box::pin(Joined(handle))
}

/// A spawned request that is aborted if nobody is waiting for it any more.
struct Joined<T>(tokio::task::JoinHandle<Result<T, ProviderError>>);

impl<T> Future for Joined<T> {
    type Output = Result<T, ProviderError>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.0).poll(cx).map(|r| match r {
            Ok(out) => out,
            Err(e) => Err(ProviderError::Api(format!("provider task ended: {e}"))),
        })
    }
}

impl<T> Drop for Joined<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[cfg(test)]
#[path = "io_tests.rs"]
mod tests;
