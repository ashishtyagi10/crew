//! A swarm worker's answer as the pane is sent it.
use std::collections::HashMap;

use super::translate;
use crate::PluginEvent;
use crew_hive::{AgentId, HiveEvent};

fn said(text: &str) -> Vec<(String, String)> {
    let ev = HiveEvent::OutputChunk {
        agent: AgentId(7),
        text: text.into(),
    };
    let out = translate(
        &ev,
        &HashMap::new(),
        &mut HashMap::new(),
        &mut HashMap::new(),
        0,
    );
    (out.into_iter())
        .filter_map(|e| match e {
            PluginEvent::Message { sender, text, .. } => Some((sender, text)),
            _ => None,
        })
        .collect()
}

/// The harness's `! not run:` line under a worker's answer is smith's row
/// under the worker's message, as on the relay, not the worker's words; a
/// keyless worker's `@tool` line is its own text and stays.
#[test]
fn a_workers_not_run_line_is_smiths_row() {
    let note = "! not run: sys:git log \u{2014} the turn ended first";
    let got = said(&format!("Looked.\n\n{note}"));
    assert_eq!(got.len(), 2, "{got:?}");
    assert_eq!(got[0].1, "Looked.");
    assert_eq!(got[1], ("agent smith".to_string(), note.to_string()));
    let keyless = "Try:\n@tool sys:git {\"cmd\": \"log\"}";
    assert_eq!(said(keyless).len(), 1);
    assert_eq!(said(keyless)[0].1, keyless);
}
