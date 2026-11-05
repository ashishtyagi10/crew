use std::time::Duration;

use crew_hive::claudestream::{parse_line, Fold};

use crate::protocol::PluginEvent;

/// Claude Code's `result` line (trimmed from a live `claude -p` 2.1.x run,
/// 2026-11-04) names its model's window; the relay's reply stat carries it to
/// the pane, the one place that window can come from.
#[test]
fn a_claude_code_reply_stat_carries_the_window_its_result_line_said() {
    let line = r#"{"type":"result","subtype":"success","is_error":false,"result":"hi","usage":{"input_tokens":2,"cache_creation_input_tokens":19353,"cache_read_input_tokens":11266,"output_tokens":18},"modelUsage":{"claude-opus-5-5":{"inputTokens":2,"outputTokens":18,"cacheReadInputTokens":11266,"cacheCreationInputTokens":19353,"contextWindow":1000000}}}"#;
    let mut fold = Fold::default();
    fold.push(parse_line(line).as_ref(), line);
    let (_, usage) = super::super::claudelane::finish(fold, "claude", "").unwrap();
    let PluginEvent::Stats { ctx, window, .. } =
        super::with_usage("claude", Duration::from_millis(5), &usage)
    else {
        panic!("a stat");
    };
    assert_eq!((ctx, window), (30_621, 1_000_000));
    let PluginEvent::Stats { window, .. } = super::latency_only("claude", Duration::ZERO) else {
        panic!("a stat");
    };
    assert_eq!(window, 0);
}
