//! A compaction keeps the finished work: what was done, what was ruled out
//! and what is left survive into the summary every later hop works from,
//! and the summary call runs at the model's own effort, not the one-shots'
//! low.
use std::sync::{Arc, Mutex};

use super::*;

/// A history with an edit that landed, a command and its result, and an
/// approach that failed.
fn history() -> Vec<String> {
    [
        "coder: @tool sys:edit_file src/lib.rs — renamed parse to parse_line",
        "coder: @tool sys:run cargo test -p demo — 12 passed",
        "coder: tried bumping serde to 2.0 — failed: E0277 in de.rs, reverted",
        "reviewer: still need to update the README example",
    ]
    .map(String::from)
    .to_vec()
}

#[test]
fn the_summary_request_asks_for_done_ruled_out_and_open_lines() {
    let req = request(Some("done: created src/lib.rs"), &history());
    for label in ["done:", "ruled out:", "open:"] {
        assert!(req.contains(label), "no `{label}` asked for:\n{req}");
    }
    assert!(
        req.contains("tool name"),
        "tool names not asked for:\n{req}"
    );
    assert!(req.contains("failed or was rejected, and why"), "{req}");
    // The history and the summary so far both reach the model.
    assert!(req.contains("serde to 2.0 — failed"), "{req}");
    assert!(req.contains("done: created src/lib.rs"), "{req}");
}

/// A verbose reply whose labelled lines sit at its tail, where a plain clip
/// cuts.
fn verbose_reply() -> String {
    format!(
        "{}\ndone: sys:edit_file src/lib.rs renamed parse to parse_line; \
         sys:run cargo test -p demo 12 passed\n\
         ruled out: serde 2.0 bump — E0277 in de.rs\n\
         - open: update the README example\n",
        "The agents talked at length about many things. ".repeat(60)
    )
}

#[test]
fn an_over_long_summary_keeps_its_labelled_lines_over_chatter() {
    let out = fit(&verbose_reply(), SUMMARY_CAP);
    assert!(out.chars().count() <= SUMMARY_CAP, "{} chars", out.len());
    for kept in [
        "done: sys:edit_file",
        "ruled out: serde 2.0",
        "open: update",
    ] {
        assert!(out.contains(kept), "`{kept}` lost:\n{out}");
    }
    assert!(!out.contains("talked at length"), "{out}");
    assert!(!out.contains('\n'), "the block stays one line: {out}");
}

#[test]
fn a_summary_that_fits_or_has_no_labels_is_clipped_as_before() {
    let short = "done: a.rs\nopen: b.rs";
    assert_eq!(fit(short, SUMMARY_CAP), clip(short, SUMMARY_CAP));
    let prose = "x ".repeat(5000);
    assert_eq!(fit(&prose, SUMMARY_CAP), clip(&prose, SUMMARY_CAP));
}

/// Through the relay's compactor: the folded block a late hop sees still
/// says what failed and what is open.
#[test]
fn the_compact_block_carries_ruled_out_and_open_into_the_next_hop() {
    let reply = verbose_reply();
    let call: Summarize = Arc::new(move |_| Ok(reply.clone()));
    let mut c = Compactor::new(Some(call));
    let mut transcript = history();
    transcript.extend((0..KEEP).map(|n| format!("entry {n}")));
    let out = c.tail(&mut transcript);
    let block = out.lines().next().unwrap();
    assert!(
        block.starts_with("[compacted 4 earlier messages:"),
        "{block}"
    );
    assert!(block.contains("ruled out: serde 2.0"), "{block}");
    assert!(block.contains("open: update the README"), "{block}");
}

/// Records the request the summarizer sends.
struct Seen(Mutex<Option<crew_hive::CompletionRequest>>);

impl crew_hive::Provider for Seen {
    fn complete(
        &self,
        req: crew_hive::CompletionRequest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<crew_hive::Completion, crew_hive::ProviderError>,
                > + Send,
        >,
    > {
        *self.0.lock().unwrap() = Some(req);
        let c = crew_hive::Completion {
            text: "done: x".into(),
            ..Default::default()
        };
        Box::pin(async move { Ok(c) })
    }
}

/// S6 sent every one-shot at low effort, the summary included; the summary
/// is the one whose quality later hops live on, so it asks for no effort.
#[test]
fn the_compaction_call_runs_at_the_models_default_effort() {
    let seen = Arc::new(Seen(Mutex::new(None)));
    let summarize = summarizer_on(seen.clone(), "claude-opus-5-5".into());
    assert_eq!(summarize(&request(None, &history())), Ok("done: x".into()));
    let req = seen.0.lock().unwrap().take().expect("the call was made");
    assert_eq!(req.effort, None, "the summary must not ask for low effort");
    assert_eq!(req.max_tokens, SUMMARY_MAX_TOKENS);
    assert!(req.prompt.contains("ruled out:"), "{}", req.prompt);
}
