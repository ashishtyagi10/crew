use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::*;

#[test]
fn a_scope_holds_the_flag_and_puts_back_what_was_there() {
    assert!(current().is_none());
    let outer = Arc::new(AtomicBool::new(false));
    let _a = enter(Arc::clone(&outer));
    {
        let inner = Arc::new(AtomicBool::new(true));
        let _b = enter(inner);
        assert!(current().unwrap().load(Ordering::Relaxed));
    }
    assert!(Arc::ptr_eq(&current().unwrap(), &outer));
}

/// A stop ends a long wait at once; with no stop the work's answer comes back.
#[test]
fn a_stop_ends_the_wait() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let flag = Arc::new(AtomicBool::new(false));
    let setter = Arc::clone(&flag);
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(150));
        setter.store(true, Ordering::Relaxed);
    });
    let started = Instant::now();
    let long = async { tokio::time::sleep(Duration::from_secs(30)).await };
    assert!(rt.block_on(unless(Some(flag), long)).is_none());
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "{:?}",
        started.elapsed()
    );
    let quick = async { 7 };
    assert_eq!(
        rt.block_on(unless(Some(Arc::new(AtomicBool::new(false))), quick)),
        Some(7)
    );
    assert_eq!(rt.block_on(unless(None, async { 8 })), Some(8));
}

/// A model that never answers in time: the shape Esc used to wait out.
struct Slow;

impl crew_hive::Provider for Slow {
    fn complete(
        &self,
        _req: crew_hive::CompletionRequest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<crew_hive::Completion, crew_hive::ProviderError>,
                > + Send,
        >,
    > {
        Box::pin(async {
            tokio::time::sleep(Duration::from_secs(60)).await;
            Ok(crew_hive::Completion::default())
        })
    }
}

/// A relay hop's model call, made from a plain worker thread as the broker
/// makes it, ends as soon as its task is stopped — not at its deadline.
#[test]
fn a_stopped_task_ends_its_model_call() {
    use crate::broker::adapter::Adapter;
    let adapter =
        crate::broker::apiadapter::ApiAdapter::new("coder", "m", "", None, Arc::new(Slow)).unwrap();
    let flag = Arc::new(AtomicBool::new(false));
    let _scope = enter(Arc::clone(&flag));
    let setter = Arc::clone(&flag);
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        setter.store(true, Ordering::Relaxed);
    });
    let started = Instant::now();
    let got = adapter.call_with_usage("hi", Duration::from_secs(30));
    assert_eq!(got.map(|(t, _)| t), Err(STOPPED.to_string()));
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "{:?}",
        started.elapsed()
    );
}
