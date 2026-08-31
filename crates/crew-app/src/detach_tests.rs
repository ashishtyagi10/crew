use super::*;

#[test]
fn detects_both_foreground_spellings() {
    assert!(has_foreground_flag(["--no-detach".to_string()]));
    assert!(has_foreground_flag(["--foreground".to_string()]));
    assert!(has_foreground_flag([
        "run".to_string(),
        "--foreground".to_string()
    ]));
    // The legacy detach flags no longer opt out of anything.
    assert!(!has_foreground_flag([
        "--detach".to_string(),
        "-d".to_string()
    ]));
    assert!(!has_foreground_flag(Vec::<String>::new()));
}

#[test]
fn strips_only_the_detach_flags() {
    let args = [
        "-d".to_string(),
        "--no-detach".to_string(),
        "--self-update".to_string(),
        "x".to_string(),
    ];
    assert_eq!(strip_detach_flags(args), vec!["--self-update", "x"]);
    let clean = ["--broker-plugin".to_string()];
    assert_eq!(strip_detach_flags(clean.clone()), vec!["--broker-plugin"]);
}

#[test]
fn restart_reexecs_the_installed_binary_path() {
    let (exe, args) = restart_command().unwrap();
    // self_update atomically replaces the file at current_exe()'s path, so
    // re-execing that path is what makes /update's relaunch load the
    // newest install.
    assert_eq!(exe, std::env::current_exe().unwrap());
    assert!(
        !args
            .iter()
            .any(|a| a == "--detach" || a == "-d" || a == "--no-detach" || a == "--foreground"),
        "detach flags must be stripped: {args:?}"
    );
}
