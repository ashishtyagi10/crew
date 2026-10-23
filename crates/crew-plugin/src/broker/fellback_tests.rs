use std::sync::Mutex;
use std::time::Duration;

use super::detail;
use crate::broker::adapter::Usage;
use crate::broker::relay::relay_turn;
use crate::broker::Adapter;
use crate::{Broker, PluginEvent, Registry};

const FELL: &str = "qwen-flash \u{b7} qwen-max: The model `qwen-max` does not exist";

fn answered(model: &str, fell_back: &str) -> crew_hive::Completion {
    crew_hive::Completion {
        model: model.into(),
        fell_back: fell_back.into(),
        input_tokens: 1000,
        output_tokens: 100,
        ..Default::default()
    }
}

/// The model that answered comes first, then what its chain skipped.
#[test]
fn the_detail_names_the_answering_model_then_the_skipped_one() {
    let skipped = "qwen-max: The model `qwen-max` does not exist";
    assert_eq!(detail(&answered("qwen-flash", skipped)), FELL);
    assert_eq!(detail(&answered("qwen-flash", "")), "", "nothing skipped");
}

/// A reply the chain fell through to is billed at the model that answered
/// — qwen-plus at $0.4/$1.2 per M is 400 + 120 µ$, where qwen-max's rates
/// would read 1600 + 640 — and its usage says what was skipped.
#[test]
fn usage_is_priced_at_the_model_that_answered_and_says_what_fell() {
    let u = Usage::of("qwen-max", &answered("qwen-plus", "qwen-max: gone"));
    assert_eq!(u.cost_microusd, 400 + 120);
    assert_eq!(u.fell_back, "qwen-plus \u{b7} qwen-max: gone");
}

/// A later dial of the same hop that its first model answered takes over
/// the context fill, but not the fallback an earlier dial made.
#[test]
fn the_latest_dial_keeps_an_earlier_fallback() {
    let mut hop = Usage {
        input_tokens: 10,
        fell_back: FELL.into(),
        ..Default::default()
    };
    hop.latest(Usage {
        input_tokens: 99,
        ..Default::default()
    });
    assert_eq!((hop.input_tokens, hop.fell_back.as_str()), (99, FELL));
}

/// Replies in order, repeating the last, each one reporting `fell`.
struct Falls(&'static str, Vec<&'static str>, &'static str, Mutex<usize>);

impl Adapter for Falls {
    fn name(&self) -> &str {
        self.0
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, _b: &str, _t: Duration) -> Result<String, String> {
        unreachable!("the relay asks for usage")
    }
    fn call_with_usage(&self, _b: &str, _t: Duration) -> Result<(String, Usage), String> {
        let mut i = self.3.lock().unwrap();
        let r = self.1.get(*i).or(self.1.last()).unwrap().to_string();
        *i += 1;
        let u = Usage {
            fell_back: self.2.into(),
            ..Default::default()
        };
        Ok((r, u))
    }
}

/// Every message of one relay turn, `(sender, text)`.
fn turn(agents: Vec<Falls>, start: &str) -> Vec<(String, String)> {
    let agents = agents
        .into_iter()
        .map(|a| Box::new(a) as Box<dyn Adapter>)
        .collect();
    let broker = Broker::new(Registry::new(agents), 6, Duration::from_secs(1));
    let mut out = Vec::new();
    let mut sink = |ev| {
        if let PluginEvent::Message { sender, text, .. } = ev {
            out.push((sender, text));
        }
        Ok(())
    };
    let tick = crate::broker::tick::noop_tick_emit();
    relay_turn(&broker, start, "task", "task", "t1", &tick, &mut sink).unwrap();
    out
}

/// `a` is dialed twice and `b` once, and every call fell through: smith
/// says it once for each agent, under that agent's first reply — not once
/// per call.
#[test]
fn smith_says_it_once_per_agent_a_turn() {
    let a = Falls(
        "a",
        vec!["draft\n@next b", "final\n@done"],
        FELL,
        Mutex::new(0),
    );
    let b = Falls("b", vec!["looked\n@next a"], FELL, Mutex::new(0));
    let msgs = turn(vec![a, b], "a");
    let line = format!("fell back to {FELL}");
    let said: Vec<usize> = (0..msgs.len())
        .filter(|&i| msgs[i] == ("agent smith".to_string(), line.clone()))
        .collect();
    assert_eq!(said.len(), 2, "one line per agent: {msgs:#?}");
    assert_eq!(
        msgs[said[0] - 1].0,
        "a \u{2192} b",
        "under a's reply: {msgs:#?}"
    );
    assert_eq!(
        msgs[said[1] - 1].0,
        "b \u{2192} a",
        "under b's reply: {msgs:#?}"
    );
}

/// A turn whose chains never fell through says nothing about them.
#[test]
fn a_turn_its_first_models_answered_says_nothing() {
    let a = Falls("a", vec!["done\n@done"], "", Mutex::new(0));
    let msgs = turn(vec![a], "a");
    assert!(
        !msgs.iter().any(|(_, t)| t.starts_with("fell back")),
        "{msgs:#?}"
    );
}
