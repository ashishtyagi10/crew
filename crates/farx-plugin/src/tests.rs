use super::*;

fn write_test_plugin(dir: &std::path::Path) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
        dir.join("hello.lua"),
        r#"
farx.register_command("hello", "Say hello", [[
    farx.message("Hello from " .. farx.current_dir)
]])
"#,
    )
    .unwrap();
}

#[test]
fn load_plugins_registers_commands() {
    let tmp = tempfile::tempdir().unwrap();
    let plugin_dir = tmp.path().join("plugins");
    write_test_plugin(&plugin_dir);

    let mut engine = PluginEngine::with_plugin_dir(plugin_dir).unwrap();
    let loaded = engine.load_plugins().unwrap();

    assert_eq!(loaded.len(), 1);
    assert!(loaded.contains(&"hello".to_string()));
    assert!(engine.has_command("hello"));
    assert_eq!(engine.list_commands().len(), 1);
}

#[test]
fn execute_command_returns_message_output() {
    let tmp = tempfile::tempdir().unwrap();
    let plugin_dir = tmp.path().join("plugins");
    write_test_plugin(&plugin_dir);

    let mut engine = PluginEngine::with_plugin_dir(plugin_dir).unwrap();
    engine.load_plugins().unwrap();

    let result = engine.execute_command("hello", "/tmp/work").unwrap();
    match result {
        PluginResult::Message(msg) => assert_eq!(msg, "Hello from /tmp/work"),
        other => panic!("expected message output, got {other:?}"),
    }
}

#[test]
fn execute_command_unknown_name_returns_error() {
    let tmp = tempfile::tempdir().unwrap();
    let plugin_dir = tmp.path().join("plugins");
    std::fs::create_dir_all(&plugin_dir).unwrap();

    let engine = PluginEngine::with_plugin_dir(plugin_dir).unwrap();
    let err = engine.execute_command("missing", "/tmp").unwrap_err();
    assert!(err.to_string().contains("Unknown plugin command"));
}
