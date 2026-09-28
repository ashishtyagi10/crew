use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// A request runs to completion when awaited from a current-thread runtime
/// that is then dropped — the work lived on the provider runtime.
#[test]
fn a_request_awaited_from_a_throwaway_runtime_completes() {
    for _ in 0..3 {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let v = rt
            .block_on(run(async { Ok::<_, ProviderError>(7) }))
            .unwrap();
        assert_eq!(v, 7);
    }
}

/// Dropping the waiting future aborts the spawned work: a caller's timeout
/// must still stop a stream from calling back.
#[test]
fn dropping_the_waiter_aborts_the_request() {
    let hits = Arc::new(AtomicUsize::new(0));
    let h = Arc::clone(&hits);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let out = rt.block_on(async move {
        tokio::time::timeout(
            Duration::from_millis(50),
            run(async move {
                loop {
                    h.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    if h.load(Ordering::SeqCst) > 1_000 {
                        return Ok::<_, ProviderError>(());
                    }
                }
            }),
        )
        .await
    });
    assert!(out.is_err(), "timed out");
    std::thread::sleep(Duration::from_millis(100));
    let after = hits.load(Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(
        hits.load(Ordering::SeqCst),
        after,
        "no callbacks after the abort"
    );
}
