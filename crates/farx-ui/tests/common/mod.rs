//! Shared helpers for integration tests.
//!
//! Lives under `tests/common/` (not `_test.rs`) so Cargo treats it as a module
//! rather than a standalone integration test binary.

use farx_core::AppConfig;
use farx_ui::app::App;

use std::sync::Mutex;

// Serialize tests that change cwd — can't safely race on a process-global
pub static CWD_LOCK: Mutex<()> = Mutex::new(());

#[allow(dead_code)]
pub fn make_app_in(dir: &std::path::Path) -> App {
    let _guard = CWD_LOCK.lock().unwrap();
    let original = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir).unwrap();
    let config = AppConfig::default();
    let app = App::new(config).unwrap();
    std::env::set_current_dir(original).unwrap();
    app
}

#[allow(dead_code)]
pub fn setup_test_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    // Create files with different sizes and extensions
    std::fs::write(dir.path().join("alpha.rs"), "fn main() {}").unwrap();
    std::fs::write(
        dir.path().join("beta.txt"),
        "hello world and more text here",
    )
    .unwrap();
    std::fs::write(dir.path().join("gamma.rs"), "// g").unwrap();
    std::fs::write(dir.path().join("delta.py"), "print('hi')").unwrap();
    std::fs::create_dir(dir.path().join("subdir")).unwrap();
    dir
}
