//! The one end-to-end test: a real `rust-analyzer` on a real (temporary)
//! cargo project with a deliberate type error. Runs only where the server
//! runs — a rustup proxy with no component installed says so and is skipped,
//! with a note, rather than failing.
use super::*;
use crate::Diagnostic;
use std::process::Command;

/// Whether `rust-analyzer` is on PATH AND actually runs (rustup ships a
/// proxy binary even when the component is absent).
fn rust_analyzer_runs() -> bool {
    crate::servers::which("rust-analyzer").is_some()
        && Command::new("rust-analyzer")
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
}

fn temp_project() -> PathBuf {
    let d = std::env::temp_dir().join(format!("crew-lsp-e2e-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("src")).unwrap();
    std::fs::write(
        d.join("Cargo.toml"),
        "[package]\nname = \"e2e\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n",
    )
    .unwrap();
    std::fs::write(
        d.join("src/main.rs"),
        "fn main() {\n    let count: i32 = \"not a number\";\n    let twice = count * 2;\n    println!(\"{twice}\");\n}\n",
    )
    .unwrap();
    d
}

#[test]
fn rust_analyzer_reports_the_type_error_and_answers_a_hover() {
    if !rust_analyzer_runs() {
        eprintln!("note: rust-analyzer is not runnable here — e2e skipped");
        return;
    }
    let root = temp_project();
    let file = root.join("src/main.rs");
    let uri = crate::uri::from_path(&file);
    let started = Instant::now();
    let mut c = Client::spawn("rust-analyzer", &[], "rust", &root).expect("spawn");
    assert_eq!(
        crate::running::list(),
        vec![("rust".to_string(), root.clone())]
    );
    c.initialize(Duration::from_secs(20)).expect("initialize");
    c.did_open(&uri, "rust", &std::fs::read_to_string(&file).unwrap())
        .expect("didOpen");
    // The first publish can be an empty list (the server clearing the file
    // before it has analysed it); wait for one that carries the error.
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut diags = Vec::new();
    while Instant::now() < deadline {
        let left = deadline.saturating_duration_since(Instant::now());
        let Some(list) = c.wait_diagnostics(&uri, left) else {
            break;
        };
        if list.iter().any(|d| d.is_error()) {
            diags = list;
            break;
        }
    }
    let errors: Vec<&Diagnostic> = diags.iter().filter(|d| d.is_error()).collect();
    assert!(
        !errors.is_empty(),
        "no error diagnostic within 20 s; got {diags:?}"
    );
    assert_eq!(
        errors[0].range.start.line, 1,
        "the error is on line 2: {:?}",
        errors[0]
    );
    // Hover on `main` (line 0, col 3) says it is a function.
    let hover = c
        .hover(&uri, 0, 3, Duration::from_secs(15))
        .expect("hover")
        .unwrap_or_default();
    assert!(hover.contains("main"), "hover said: {hover:?}");
    // The definition of `count` at its use (line 2, col 16) is its binding.
    let defs = c
        .definition(&uri, 2, 16, Duration::from_secs(15))
        .expect("definition");
    assert!(
        defs.iter()
            .any(|l| l.range.start.line == 1 && l.path().as_deref() == Some(file.as_path())),
        "definition: {defs:?}"
    );
    eprintln!("note: rust-analyzer e2e took {:?}", started.elapsed());
    c.shutdown();
    drop(c);
    assert!(
        crate::running::list().is_empty(),
        "drop leaves the running list"
    );
    let _ = std::fs::remove_dir_all(&root);
}
