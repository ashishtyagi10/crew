use super::*;
use std::io::Cursor;

#[test]
fn the_three_kinds_are_told_apart() {
    assert_eq!(
        classify(json!({"jsonrpc": "2.0", "id": 7, "result": {"x": 1}})),
        Incoming::Response(json!({"jsonrpc": "2.0", "id": 7, "result": {"x": 1}}))
    );
    assert_eq!(
        classify(
            json!({"jsonrpc": "2.0", "method": "textDocument/publishDiagnostics", "params": {"uri": "u"}})
        ),
        Incoming::Notification(Notification {
            method: "textDocument/publishDiagnostics".into(),
            params: json!({"uri": "u"}),
        })
    );
    assert_eq!(
        classify(
            json!({"jsonrpc": "2.0", "id": "req-1", "method": "workspace/configuration", "params": {"items": []}})
        ),
        Incoming::ServerRequest {
            id: json!("req-1"),
            method: "workspace/configuration".into(),
            params: json!({"items": []}),
        }
    );
}

/// A notification without params still classifies — `params` is optional
/// on the wire — and a `null` id is a notification, not a request.
#[test]
fn a_bare_method_is_a_notification_even_with_a_null_id() {
    let n = classify(json!({"jsonrpc": "2.0", "id": null, "method": "exit"}));
    assert_eq!(
        n,
        Incoming::Notification(Notification {
            method: "exit".into(),
            params: Value::Null
        })
    );
}

#[test]
fn a_configuration_request_gets_one_null_per_item() {
    let r = reply_to(
        "workspace/configuration",
        json!(3),
        &json!({"items": [{"section": "a"}, {"section": "b"}]}),
    );
    assert_eq!(
        r,
        json!({"jsonrpc": "2.0", "id": 3, "result": [null, null]})
    );
}

#[test]
fn anything_else_the_server_asks_is_refused_as_method_not_found() {
    let r = reply_to("client/registerCapability", json!(9), &Value::Null);
    assert_eq!(r["id"], 9);
    assert_eq!(r["error"]["code"], -32601);
    assert!(r.get("result").is_none());
}

/// The reader fn over a byte stream holding all three kinds, in an order
/// that interleaves them: everything comes out, sorted, in wire order.
#[test]
fn pump_routes_a_mixed_stream_in_order_and_stops_when_asked() {
    let mut wire = Vec::new();
    wire.extend(crate::framing::encode(&json!({"id": 1, "result": null})));
    wire.extend(crate::framing::encode(&json!({"method": "n1"})));
    wire.extend(crate::framing::encode(&json!({"id": 2, "method": "ask"})));
    wire.extend(crate::framing::encode(&json!({"method": "n2"})));
    let mut seen = Vec::new();
    pump(Cursor::new(wire.clone()), |m| {
        seen.push(m);
        true
    });
    let kinds: Vec<&str> = seen
        .iter()
        .map(|m| match m {
            Incoming::Response(_) => "resp",
            Incoming::Notification(n) => n.method.as_str(),
            Incoming::ServerRequest { .. } => "req",
        })
        .collect();
    assert_eq!(kinds, ["resp", "n1", "req", "n2"]);
    // A sink that says stop after the first message is obeyed.
    let mut count = 0;
    pump(Cursor::new(wire), |_| {
        count += 1;
        false
    });
    assert_eq!(count, 1);
}

#[test]
fn pump_ends_quietly_on_garbage() {
    let mut count = 0;
    pump(Cursor::new(b"not a frame at all".to_vec()), |_| {
        count += 1;
        true
    });
    assert_eq!(count, 0);
}
