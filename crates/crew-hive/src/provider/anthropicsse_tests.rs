use super::*;
use std::sync::{Arc, Mutex};

fn sink() -> (ChunkFn, Arc<Mutex<Vec<(bool, String)>>>) {
    let got = Arc::new(Mutex::new(Vec::new()));
    let g = Arc::clone(&got);
    let f: ChunkFn = Arc::new(move |c: Chunk<'_>| {
        let (thought, s) = match c {
            Chunk::Text(s) => (false, s),
            Chunk::Thought(s) => (true, s),
        };
        g.lock().unwrap().push((thought, s.to_string()));
    });
    (f, got)
}

/// A real stream's shape (Messages API, 2023-06-01), cut at awkward places.
const STREAM: &str = "event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":42,\"output_tokens\":1}}}\n\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"weigh it\"}}\n\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}\n\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"text_delta\",\"text\":\", world\"}}\n\n\
event: message_delta\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":7}}\n\n\
event: message_stop\n\
data: {\"type\":\"message_stop\"}\n\n";

#[test]
fn a_stream_folds_into_text_thought_and_usage_forwarding_each_fragment() {
    let (f, got) = sink();
    let mut fold = Fold::default();
    // Fed in 7-byte slivers: lines split mid-JSON must still parse.
    let bytes = STREAM.as_bytes();
    for piece in bytes.chunks(7) {
        fold.feed(std::str::from_utf8(piece).unwrap(), &f);
    }
    let c = fold.finish(&f).unwrap();
    assert_eq!(c.text, "Hello, world");
    assert_eq!(c.thought, "weigh it");
    assert_eq!((c.input_tokens, c.output_tokens), (42, 7));
    let got = got.lock().unwrap().clone();
    assert_eq!(
        got,
        vec![
            (true, "weigh it".to_string()),
            (false, "Hello".to_string()),
            (false, ", world".to_string())
        ]
    );
}

#[test]
fn an_error_event_fails_the_completion() {
    let (f, _) = sink();
    let mut fold = Fold::default();
    fold.feed(
        "event: error\ndata: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"Overloaded\"}}\n\n",
        &f,
    );
    let e = fold.finish(&f).unwrap_err();
    assert!(e.to_string().contains("Overloaded"), "{e}");
}

/// A server that ignores `"stream": true` answers with the whole JSON reply:
/// read as one, forwarded as one chunk — not an empty success.
#[test]
fn a_whole_json_reply_is_read_as_one() {
    let (f, got) = sink();
    let mut fold = Fold::default();
    fold.feed(
        "{\"type\":\"message\",\"content\":[{\"type\":\"text\",\"text\":\"whole\"}],\
         \"usage\":{\"input_tokens\":3,\"output_tokens\":1}}",
        &f,
    );
    let c = fold.finish(&f).unwrap();
    assert_eq!(c.text, "whole");
    assert_eq!(
        got.lock().unwrap().clone(),
        vec![(false, "whole".to_string())]
    );
}
