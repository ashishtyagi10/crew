//! The scope and the numbers a cap is read from: environment value, dollars
//! as read, the unwrapped fail-open path, and a model with no price.
use super::*;
use crate::broker::spendcap::kit::{call, run, task, Priced, Tokens};
use crate::broker::spendcapwrap::{cost, wrap, Capped};
use crew_hive::{Completion, Provider};
use std::sync::Arc;

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
