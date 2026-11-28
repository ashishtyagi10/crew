//! Live probe against a real rust-analyzer, `#[ignore]`d so CI (which has no
//! server installed) skips it: the scripted fake in `symbols_tests.rs` proves
//! our parsing, only a real server proves the request shape and handshake.
//! Run: `cargo test -p crew-lsp symbols_live -- --ignored --nocapture`.
use std::time::{Duration, Instant};

use crate::Client;

#[test]
#[ignore = "needs rust-analyzer on PATH or ~/.cargo/bin"]
fn symbols_live_rust_analyzer_finds_a_struct_by_name() {
    let dir = std::env::temp_dir().join(format!("crew-symbols-live-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("src/lib.rs"),
        "pub struct ToolBudget;\npub fn tool_budget_left() {}\n",
    )
    .unwrap();
    let home = std::env::var("HOME").unwrap_or_default();
    let bin = format!("{home}/.cargo/bin/rust-analyzer");
    let bin = if std::path::Path::new(&bin).exists() {
        bin
    } else {
        "rust-analyzer".into()
    };
    let root = dir.canonicalize().unwrap();
    let mut c = Client::spawn(&bin, &[], "rust", &root).unwrap();
    c.initialize(Duration::from_secs(30)).unwrap();
    // rust-analyzer answers an empty list until its index is built.
    let deadline = Instant::now() + Duration::from_secs(60);
    let found = loop {
        let list = c
            .workspace_symbols("ToolBudget", Duration::from_secs(15))
            .unwrap_or_default();
        if !list.is_empty() || Instant::now() >= deadline {
            break list;
        }
        std::thread::sleep(Duration::from_millis(500));
    };
    for s in &found {
        println!("{} {} {} line={:?}", s.kind, s.name, s.uri, s.line);
    }
    let hit = found.iter().find(|s| s.name == "ToolBudget");
    let _ = std::fs::remove_dir_all(&dir);
    let hit = hit.expect("ToolBudget not found");
    assert_eq!(hit.kind, "struct");
    assert!(hit.uri.ends_with("src/lib.rs"), "{}", hit.uri);
    assert_eq!(hit.line, Some(0));
}
