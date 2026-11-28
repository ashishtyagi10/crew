//! `sys:read_file` of a picture, down the real tool surface to the picture.
use super::super::approval::Requester;
use super::super::toolcall::ToolRunner;
use super::*;

fn png(w: u32, h: u32) -> Vec<u8> {
    let mut b = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];
    b.extend_from_slice(b"IHDR");
    b.extend_from_slice(&w.to_be_bytes());
    b.extend_from_slice(&h.to_be_bytes());
    b
}

fn file(name: &str, bytes: &[u8]) -> String {
    let dir = std::env::temp_dir().join(format!("crew-sysimage-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join(name);
    std::fs::write(&p, bytes).unwrap();
    p.to_string_lossy().into_owned()
}

fn tools() -> SessionTools {
    let host = Arc::new(Mutex::new(crate::mcp::McpHost::default()));
    SessionTools::for_requester(host, true, Requester::LocalPane)
}

#[test]
fn read_file_of_a_png_is_one_line_and_a_picture() {
    let path = file("shot.png", &png(1280, 720));
    let args = serde_json::json!({ "path": path }).to_string();
    let t = tools();
    let out = t.call("sys", "read_file", &args).unwrap();
    assert_eq!(out, "image: shot.png, 1280x720");
    let shown = t.attachments("sys", "read_file", &args, &out);
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].media_type, "image/png");
    assert_eq!(shown[0].name, "shot.png");
}

#[test]
fn a_picture_wraps_through_own_hands_too() {
    let path = file("wrapped.png", &png(3, 3));
    let args = serde_json::json!({ "path": path }).to_string();
    let inner: Arc<dyn ToolRunner> = Arc::new(tools());
    let wrapped = super::super::engine::ownhands::OwnHands(inner);
    let out = "image: wrapped.png, 3x3";
    assert_eq!(wrapped.attachments("sys", "read_file", &args, out).len(), 1);
}

#[test]
fn a_repeat_pointer_or_a_text_file_attaches_nothing() {
    let path = file("again.png", &png(2, 2));
    let args = serde_json::json!({ "path": path }).to_string();
    let t = tools();
    assert!(t
        .attachments("sys", "read_file", &args, "same as round 1")
        .is_empty());
    assert!(t.attachments("sys", "grep", &args, "image: x").is_empty());
}

#[test]
fn a_png_over_the_cap_is_a_note_and_a_text_file_still_reads_as_text() {
    let mut big = png(1, 1);
    big.resize(5 * 1024 * 1024, 0);
    let path = file("big.png", &big);
    let args = serde_json::json!({ "path": path }).to_string();
    let t = tools();
    let out = t.call("sys", "read_file", &args).unwrap();
    assert!(out.starts_with("[image omitted: big.png, "), "{out}");
    assert!(t.attachments("sys", "read_file", &args, &out).is_empty());

    let fake = file("fake.png", b"just words\n");
    let args = serde_json::json!({ "path": fake }).to_string();
    assert!(t
        .call("sys", "read_file", &args)
        .unwrap()
        .contains("just words"));
}

#[test]
fn a_task_that_pastes_a_temp_path_attaches_it() {
    let path = file("crew-paste-1.png", &png(5, 5));
    let ask = format!("what is wrong with {path}?");
    let (prompt, images) = super::super::hopshots::attach(true, &ask, "FRAME".into());
    assert_eq!(images.len(), 1);
    assert_eq!(prompt, "FRAME");
    // A CLI agent opens the path itself.
    assert!(super::super::hopshots::attach(false, &ask, "FRAME".into())
        .1
        .is_empty());
    // A path that names nothing attaches nothing.
    assert!(
        super::super::hopshots::attach(true, "see /no/such/x.png", "F".into())
            .1
            .is_empty()
    );
}
