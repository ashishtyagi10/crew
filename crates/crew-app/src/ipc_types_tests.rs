use super::*;

#[test]
fn request_and_reply_round_trip() {
    let req = Request::Ask {
        v: PROTOCOL_V,
        from: "builder".into(),
        to: "schema".into(),
        question: "which API version?".into(),
        id: "q7".into(),
    };
    let json = serde_json::to_string(&req).unwrap();
    assert_eq!(serde_json::from_str::<Request>(&json).unwrap(), req);

    let na = Reply::NoAnswer {
        reason: NoAnswer::IdleNoEngage,
        partial: None,
    };
    let json = serde_json::to_string(&na).unwrap();
    assert_eq!(serde_json::from_str::<Reply>(&json).unwrap(), na);

    let ans = serde_json::to_string(&Reply::Answered { text: "hi".into() }).unwrap();
    assert!(ans.contains("Answered"), "{ans}");
}

#[test]
fn panes_request_parses_from_a_client_line() {
    let req: Request = serde_json::from_str(r#"{"op":"Panes","v":1}"#).unwrap();
    assert_eq!(req, Request::Panes { v: 1 });
}

#[test]
fn broadcast_request_and_cast_reply_round_trip() {
    let req = Request::Broadcast {
        v: PROTOCOL_V,
        from: "builder".into(),
        question: "status?".into(),
        id: "q9".into(),
        mode: CastMode::All,
    };
    let json = serde_json::to_string(&req).unwrap();
    assert_eq!(serde_json::from_str::<Request>(&json).unwrap(), req);

    let cast = Reply::Cast {
        answers: vec![CastAnswer {
            pane: "p1".into(),
            label: Some("schema".into()),
            text: Some("done".into()),
            no_answer: None,
        }],
    };
    let json = serde_json::to_string(&cast).unwrap();
    assert_eq!(serde_json::from_str::<Reply>(&json).unwrap(), cast);
}
