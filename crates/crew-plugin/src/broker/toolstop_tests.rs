//! A stop pressed while a relay agent's tool runs: no follow-up dial, and the
//! answer says the turn was stopped and how far it got. The tool presses
//! stop itself, as a person would while a build ran, so the moment is exact.
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::*;
use crate::broker::hop::HopKind;
use crate::broker::Adapter;
use crate::{PluginEvent, Registry};

/// Asks for the build on every reply, and records every prompt it is dialed
/// with — the count of those IS the model calls made.
struct Builder {
    name: &'static str,
    dials: Arc<Mutex<Vec<String>>>,
}

impl Adapter for Builder {
    fn name(&self) -> &str {
        self.name
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, body: &str, _t: Duration) -> Result<String, String> {
        self.dials.lock().unwrap().push(body.to_string());
        Ok("building now\n@tool fs:build {}".into())
    }
}

/// A tool whose every call presses stop, and which counts its runs.
struct Stopping(Arc<AtomicBool>, Arc<AtomicUsize>);

impl ToolRunner for Stopping {
    fn hint(&self) -> String {
        "TOOLS: fs:build".into()
    }
    fn call(&self, _server: &str, _tool: &str, _args: &str) -> Result<String, String> {
        self.1.fetch_add(1, Ordering::SeqCst);
        self.0.store(true, Ordering::Relaxed);
        Ok("built".into())
    }
}

/// A broker over one scripted agent (and a peer, so the hop is mid-chain and
/// a repair re-ask would be possible) whose tool presses stop.
fn stopping_broker() -> (Broker, Arc<Mutex<Vec<String>>>, Arc<AtomicUsize>) {
    let dials = Arc::new(Mutex::new(Vec::new()));
    let peer = Arc::new(Mutex::new(Vec::new()));
    let flag = Arc::new(AtomicBool::new(false));
    let ran = Arc::new(AtomicUsize::new(0));
    let registry = Registry::new(vec![
        Box::new(Builder {
            name: "planner",
            dials: Arc::clone(&dials),
        }),
        Box::new(Builder {
            name: "coder",
            dials: peer,
        }),
    ]);
    let broker = Broker::new(registry, 6, Duration::from_secs(5))
        .with_tools(Arc::new(Stopping(Arc::clone(&flag), Arc::clone(&ran))))
        .with_cancel_flag(flag);
    (broker, dials, ran)
}

#[test]
fn a_stop_during_round_one_dials_no_round_two() {
    let (b, dials, ran) = stopping_broker();
    let agent = Builder {
        name: "planner",
        dials: Arc::clone(&dials),
    };
    let mut hops = Vec::new();
    let reply = b.run_tools(
        &agent,
        "base",
        "building now\n@tool fs:build {}".into(),
        &mut RunStats::default(),
        &mut Usage::default(),
        &Envelope::new("user", "planner", "t1", "task"),
        &HopStream::noop(),
        &mut |h| hops.push(h),
    );
    assert_eq!(ran.load(Ordering::SeqCst), 1, "round one's tool ran");
    assert!(
        dials.lock().unwrap().is_empty(),
        "a round-two model call went out after the stop"
    );
    assert_eq!(reply, "building now\n\nstopped \u{2014} 1 tool call made");
}

/// Through the whole engine, mid-chain: the stopped reply is the answer, and
/// no repair re-ask goes out to add the directive it lacks.
#[test]
fn a_stopped_turn_ends_on_its_note_with_no_repair_dial() {
    let (b, dials, ran) = stopping_broker();
    let tick: Arc<dyn Fn(PluginEvent) + Send + Sync> = Arc::new(|_| {});
    let mut hops = Vec::new();
    b.run("coder", "planner", "build it", "t1", &tick, &mut |h| {
        hops.push(h)
    });
    assert_eq!(ran.load(Ordering::SeqCst), 1);
    assert_eq!(dials.lock().unwrap().len(), 1, "only the first dial");
    let done = hops.iter().find(|h| h.kind == HopKind::Done);
    let done = done.unwrap_or_else(|| panic!("no answer: {hops:?}"));
    assert!(
        done.text.ends_with("stopped \u{2014} 1 tool call made"),
        "{}",
        done.text
    );
}
