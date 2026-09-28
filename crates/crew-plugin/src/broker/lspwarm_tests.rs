use std::collections::BTreeMap;

use serde_json::json;

use super::*;

/// A warmer that only writes down what it was asked to start.
#[derive(Default)]
struct Asked(Mutex<Vec<PathBuf>>);

impl Warmer for Asked {
    fn warm(&self, path: &Path) {
        self.0.lock().unwrap().push(path.to_path_buf());
    }
}

impl Asked {
    fn paths(&self) -> Vec<PathBuf> {
        self.0.lock().unwrap().clone()
    }
}

fn record() -> (Arc<Asked>, Arc<WarmOnWrite>) {
    let asked = Arc::new(Asked::default());
    let warm = WarmOnWrite::new(Arc::clone(&asked) as Arc<dyn Warmer>);
    (asked, warm)
}

/// One start per language per task: the first write asks, the rest of the
/// task's writes in that language do not, and a read never does.
#[test]
fn the_first_write_of_a_language_warms_it_and_nothing_else_does() {
    let (asked, w) = record();
    w.wrote("read_file", r#"{"path": "src/lib.rs"}"#);
    assert!(asked.paths().is_empty(), "a read starts nothing");
    w.wrote("edit", r#"{"path": "src/lib.rs", "old": "a", "new": "b"}"#);
    assert_eq!(asked.paths(), [PathBuf::from("src/lib.rs")]);
    w.wrote("write_file", r#"{"path": "src/main.rs", "content": ""}"#);
    w.wrote("edit", r#"{"path": "src/lib.rs", "old": "b", "new": "c"}"#);
    assert_eq!(asked.paths().len(), 1, "one start per language per task");
    // Nobody serves markdown; a second language gets its own start.
    w.wrote("write_file", r#"{"path": "notes.md", "content": ""}"#);
    w.wrote("write_file", r#"{"path": "tool.py", "content": ""}"#);
    assert_eq!(
        asked.paths(),
        [PathBuf::from("src/lib.rs"), PathBuf::from("tool.py")]
    );
}

/// Each task asks again: the host may have dropped the client since, and a
/// start that finds the server running costs nothing.
#[test]
fn the_next_task_asks_again() {
    let (asked, w) = record();
    let edit = r#"{"path": "src/lib.rs", "old": "a", "new": "b"}"#;
    w.wrote("edit", edit);
    let next = w.fresh();
    next.wrote("edit", edit);
    w.wrote("edit", edit);
    assert_eq!(asked.paths().len(), 2, "{:?}", asked.paths());
}

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "crew-lspwarm-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Through the real tool surface, which decides what a write is and whether
/// it happened: a task's first Rust write asks once, its second does not, a
/// read never does, and a write that failed starts nothing.
#[test]
fn a_tasks_first_rust_write_through_the_surface_warms_once() {
    let dir = temp_dir();
    let (a, b) = (dir.join("a.rs"), dir.join("b.rs"));
    std::fs::write(&a, "fn a() {}\n").unwrap();
    let (asked, w) = record();
    let t = crate::broker::session::sessiontest::sys_surface_warming(Arc::clone(&w));
    let call = |tool: &str, args: serde_json::Value| t.call("sys", tool, &args.to_string());

    call("read_file", json!({"path": a})).unwrap();
    assert!(asked.paths().is_empty(), "a read starts nothing");
    call(
        "edit",
        json!({"path": a, "old": "no such text", "new": "x"}),
    )
    .unwrap_err();
    assert!(asked.paths().is_empty(), "a failed edit starts nothing");
    call("write_file", json!({"path": b, "content": "fn b() {}\n"})).unwrap();
    assert_eq!(asked.paths(), [b.as_path()]);
    call("edit", json!({"path": a, "old": "fn a", "new": "pub fn a"})).unwrap();
    assert_eq!(
        asked.paths(),
        [b.as_path()],
        "the second write does not ask again"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A server that is not installed is not started, and says nothing: the
/// task's end would not have mentioned it either (`serves_file`), and a LOG
/// line on every task that writes would be noise about a choice the user made.
#[test]
fn warming_a_server_that_is_not_installed_notes_nothing() {
    let mut h = crate::lsp::LspHost::new(BTreeMap::from([(
        "rust".to_string(),
        crew_lsp::servers::Server {
            command: "crew-no-such-ls".into(),
            args: vec![],
        },
    )]));
    let notes: Arc<Mutex<Vec<String>>> = Arc::default();
    let n = Arc::clone(&notes);
    h.set_sinks(
        Arc::new(move |_, m: &str| n.lock().unwrap().push(m.to_string())),
        Arc::new(|_| {}),
    );
    h.warm(Path::new("src/lib.rs"));
    h.warm(Path::new("notes.md"));
    assert!(notes.lock().unwrap().is_empty(), "{notes:?}");
}
