use super::*;

#[test]
fn parse_reads_the_mcp_servers_map() {
    let m = parse(
        r#"{"mcpServers":{"fs":{"command":"mcp-fs","args":["--root","."],"env":{"K":"v"}}}}"#,
    );
    let fs = &m["fs"];
    assert_eq!(fs.command, "mcp-fs");
    assert_eq!(fs.args, vec!["--root", "."]);
    assert_eq!(fs.env["K"], "v");
}

#[test]
fn parse_defaults_args_and_env() {
    let m = parse(r#"{"mcpServers":{"x":{"command":"srv"}}}"#);
    assert!(m["x"].args.is_empty());
    assert!(m["x"].env.is_empty());
}

#[test]
fn parse_of_garbage_or_empty_is_empty() {
    assert!(parse("not json").is_empty());
    assert!(parse("{}").is_empty());
}

#[test]
fn load_file_of_missing_path_is_empty() {
    assert!(load_file(Path::new("/nonexistent/mcp.json")).is_empty());
}
