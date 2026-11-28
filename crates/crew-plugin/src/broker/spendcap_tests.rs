//! The spend cap: pause between calls, ask once, a yes raises, a no ends.
//! Answers are delivered from inside the injected asker, so no emitter is
//! installed (`approval::set_emitter` is process-wide) and nothing waits on a
//! clock except the tests that mean to.
use super::*;
use crate::broker::spendcapwrap::{cost, wrap, Capped};
use crew_hive::{Completion, CompletionRequest, Provider, ProviderError};
use std::sync::atomic::AtomicUsize;
use std::sync::Mutex;

type Reply =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<Completion, ProviderError>> + Send>>;

/// A provider that answers every call with `micros` of cost.
struct Priced {
    micros: u64,
    calls: AtomicUsize,
}

impl Priced {
    fn of(micros: u64) -> Arc<Self> {
        Arc::new(Self {
            micros,
            calls: AtomicUsize::new(0),
        })
    }
    fn made(&self) -> usize {
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
struct Pane {
    asked: Mutex<Vec<String>>,
    said: Mutex<Vec<String>>,
    cancel: Arc<AtomicBool>,
}

/// A task capped at `cap` micro-USD whose asker answers `answer` at once
/// (`None`: nobody answers).
fn task(cap: u64, answer: Option<bool>) -> (Arc<TaskSpend>, Arc<Pane>) {
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

fn run<T>(f: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(f)
}

fn capped(inner: &Arc<Priced>, task: &Arc<TaskSpend>) -> Capped {
    Capped {
        inner: Arc::clone(inner) as Arc<dyn Provider>,
        model: "scripted".into(),
        task: Arc::clone(task),
    }
}

async fn call(p: &Capped) -> Result<Completion, ProviderError> {
    p.complete(CompletionRequest::default()).await
}

#[test]
fn under_the_cap_nothing_pauses() {
    let (t, pane) = task(10_000_000, Some(true));
    let m = Priced::of(1_000_000);
    let c = capped(&m, &t);
    run(async {
        for _ in 0..9 {
            call(&c).await.unwrap();
        }
    });
    assert_eq!(m.made(), 9);
    assert!(pane.asked.lock().unwrap().is_empty(), "asked under the cap");
    assert_eq!(t.spent(), 9_000_000);
}

#[test]
fn crossing_the_cap_asks_once_between_calls_and_a_yes_raises_it() {
    let (t, pane) = task(10_000_000, Some(true));
    let m = Priced::of(5_000_000);
    let c = capped(&m, &t);
    run(async {
        // Two calls reach $10; the third is the one held, never the second.
        for _ in 0..3 {
            call(&c).await.unwrap();
        }
    });
    assert_eq!(m.made(), 3, "a yes lets the call through");
    let asked = pane.asked.lock().unwrap();
    assert_eq!(asked.len(), 1, "asked once: {asked:?}");
    assert!(
        asked[0].starts_with("spent $10 of the $10 cap") && asked[0].contains("raises it by $10"),
        "{}",
        asked[0]
    );
    assert!(pane.said.lock().unwrap().is_empty());
    assert!(!pane.cancel.load(Ordering::Relaxed));
    assert_eq!(
        t.cap.load(Ordering::Relaxed),
        20_000_000,
        "raised by one step"
    );
}

#[test]
fn a_no_ends_the_task_with_one_line_and_asks_no_more() {
    let (t, pane) = task(10_000_000, Some(false));
    let m = Priced::of(5_200_000);
    let c = capped(&m, &t);
    let out = run(async {
        call(&c).await.unwrap();
        call(&c).await.unwrap(); // $10.40
        let held = call(&c).await;
        let again = call(&c).await;
        (held, again)
    });
    assert!(out.0.is_err() && out.1.is_err());
    assert_eq!(m.made(), 2, "no call after the no");
    assert_eq!(pane.asked.lock().unwrap().len(), 1);
    assert_eq!(
        *pane.said.lock().unwrap(),
        vec!["stopped at the spend cap ($10.40)".to_string()]
    );
    assert!(pane.cancel.load(Ordering::Relaxed), "the task is stopped");
}

#[test]
fn silence_is_a_no() {
    let (t, pane) = task(1_000_000, None);
    t.add(1_000_000);
    let c = capped(&Priced::of(1), &t);
    assert!(run(call(&c)).is_err());
    assert_eq!(pane.said.lock().unwrap().len(), 1);
    assert!(pane.cancel.load(Ordering::Relaxed));
}

#[test]
fn stop_while_paused_ends_it_without_the_cap_line() {
    let (t, pane) = task(1_000_000, None);
    t.add(1_000_000);
    let c = capped(&Priced::of(1), &t);
    let flag = Arc::clone(&pane.cancel);
    let started = Instant::now();
    let held = run(async {
        let stopper = async {
            tokio::time::sleep(Duration::from_millis(30)).await;
            flag.store(true, Ordering::Relaxed);
        };
        tokio::join!(call(&c), stopper).0
    });
    assert!(held.is_err());
    assert!(started.elapsed() < Duration::from_millis(140), "waited out");
    assert!(
        pane.said.lock().unwrap().is_empty(),
        "a stop is not the cap"
    );
}

#[test]
fn cap_zero_never_pauses() {
    assert_eq!(parse(Some("0")), 0);
    assert_eq!(parse(Some("-3")), 0);
    let none = TaskSpend::new(0, Arc::default(), Arc::new(|_, _| true), Arc::new(|_| {}));
    assert!(none.is_none(), "a zero cap is no meter at all");
    // And with no meter entered, a provider is handed back unwrapped.
    let m: Arc<dyn Provider> = Priced::of(u64::MAX / 4);
    let w = wrap(Arc::clone(&m), "x");
    assert!(Arc::ptr_eq(&m, &w));
}

#[test]
fn a_model_with_no_price_counts_zero_and_never_reaches_the_cap() {
    let tokens = Completion {
        input_tokens: 9_000_000,
        output_tokens: 9_000_000,
        ..Default::default()
    };
    assert_eq!(cost("no-such-model-anywhere", &tokens), 0);
    let priced = Completion {
        input_tokens: 1_000_000,
        ..Default::default()
    };
    assert!(
        cost("claude-sonnet-4-5", &priced) > 0,
        "a listed model is priced"
    );
    let (t, pane) = task(1_000_000, Some(false));
    let free = Arc::new(Tokens);
    let c = Capped {
        inner: free,
        model: "no-such-model-anywhere".into(),
        task: Arc::clone(&t),
    };
    run(async {
        for _ in 0..5 {
            call(&c).await.unwrap();
        }
    });
    assert_eq!(t.spent(), 0);
    assert!(pane.asked.lock().unwrap().is_empty());
}

/// Tokens and no price.
struct Tokens;

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

#[test]
fn agents_of_a_swarm_share_one_cap_and_one_question() {
    let (t, pane) = task(10_000_000, Some(true));
    let (a, b) = (Priced::of(4_000_000), Priced::of(7_000_000));
    let (ca, cb) = (capped(&a, &t), capped(&b, &t));
    run(async {
        // Each agent alone is under $10; together they are over, and the
        // two that arrive at once open one question between them.
        call(&ca).await.unwrap();
        call(&cb).await.unwrap();
        let (x, y) = tokio::join!(call(&ca), call(&cb));
        x.unwrap();
        y.unwrap();
    });
    assert_eq!(
        pane.asked.lock().unwrap().len(),
        1,
        "one question for the task"
    );
    assert_eq!(a.made() + b.made(), 4);
}

#[test]
fn the_scope_hands_the_task_its_meter_and_restores() {
    assert!(current().is_none());
    let (t, _) = task(1_000_000, None);
    {
        let _s = enter(Some(Arc::clone(&t)));
        assert!(Arc::ptr_eq(&current().unwrap(), &t));
        let m: Arc<dyn Provider> = Priced::of(1);
        assert!(!Arc::ptr_eq(&m, &wrap(Arc::clone(&m), "x")), "wrapped");
    }
    assert!(current().is_none());
}

#[test]
fn the_environment_value_and_the_dollars_a_person_reads() {
    assert_eq!(parse(None), 10_000_000);
    assert_eq!(parse(Some("2.5")), 2_500_000);
    assert_eq!(parse(Some("lots")), 10_000_000);
    assert_eq!(parse(Some("NaN")), 10_000_000);
    assert_eq!(usd(10_000_000), "$10");
    assert_eq!(usd(10_400_000), "$10.40");
    assert_eq!(usd(214_000), "$0.21");
}
