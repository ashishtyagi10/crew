use super::*;

fn spec() -> CliSpec {
    super::super::registry::by_name("codex")
        .unwrap()
        .cli
        .unwrap()
}

/// The exit code is the primary signal, markers the guard: the table is the
/// real output of `claude auth status` / `codex login status` on 2026-08-01
/// installs, plus the two ways an exit code can lie.
#[test]
fn probe_reads_signed_in_and_out_from_the_clis_own_words() {
    let table: &[(&str, bool, &str, CliAuth)] = &[
        (
            "codex signed in",
            true,
            "Logged in using ChatGPT",
            CliAuth::SignedIn,
        ),
        (
            "claude signed in",
            true,
            r#"{"loggedIn": true, "subscriptionType": "max"}"#,
            CliAuth::SignedIn,
        ),
        (
            "codex signed out",
            false,
            "Not logged in",
            CliAuth::SignedOut,
        ),
        (
            "claude signed out politely",
            true,
            r#"{"loggedIn": false}"#,
            CliAuth::SignedOut,
        ),
        (
            "plain refusal",
            false,
            "error: not authenticated",
            CliAuth::SignedOut,
        ),
        (
            "an old cli without the subcommand",
            false,
            "error: unrecognized subcommand 'status'\nUsage: codex [OPTIONS]",
            CliAuth::Unknown,
        ),
    ];
    for (case, ok, output, want) in table {
        let got = probe_with(&spec(), true, &|_, _| Some((*ok, (*output).to_string())));
        assert_eq!(got, *want, "{case}");
    }
}

#[test]
fn a_missing_binary_is_absent_and_never_spawned() {
    let got = probe_with(&spec(), false, &|_, _| {
        panic!("must not spawn a status command for an uninstalled CLI")
    });
    assert_eq!(got, CliAuth::Absent);
}

#[test]
fn a_spawn_failure_or_timeout_reads_unknown() {
    assert_eq!(probe_with(&spec(), true, &|_, _| None), CliAuth::Unknown);
}

/// The probe asks the CLI's status argv, not something else — the registry's
/// data is what reaches the child process.
#[test]
fn the_probe_runs_the_registry_status_argv() {
    let seen = std::cell::RefCell::new(Vec::<String>::new());
    let spec = spec();
    probe_with(&spec, true, &|bin, args| {
        seen.borrow_mut().push(format!("{bin} {}", args.join(" ")));
        Some((true, "Logged in".into()))
    });
    assert_eq!(*seen.borrow(), ["codex login status"]);
}

/// End to end through a real child process: the bounded runner keeps the
/// exit STATUS (unlike `run::run_cli`, for which empty stdout is an error),
/// and a hung child is killed at the deadline.
#[test]
fn run_status_reports_exit_and_output_and_kills_a_hang() {
    let ok = run_status("sh", &["-c", "printf 'Logged in'"], Duration::from_secs(5));
    assert_eq!(ok, Some((true, "Logged in".into())));
    let no = run_status(
        "sh",
        &["-c", "printf 'Not logged in' >&2; exit 1"],
        Duration::from_secs(5),
    );
    assert_eq!(no, Some((false, "Not logged in".into())));
    let hung = run_status("sh", &["-c", "sleep 5"], Duration::from_millis(150));
    assert_eq!(hung, None);
    assert_eq!(
        run_status("definitely-not-a-binary-xyz", &[], Duration::from_secs(1)),
        None
    );
}

/// `ant auth status` exits 0 whether or not a profile exists, so its spec
/// names a marker instead: the real signed-out text of ant 1.31.0 (no
/// profile configured) reads signed OUT, the documented signed-in row reads
/// signed IN, and a build without the subcommand is no verdict at all.
#[test]
fn a_marker_spec_ignores_the_exit_code() {
    let spec = super::super::mint::ANT.cli;
    let signed_out = "Active profile:  default (fallback \u{2014} no active_config set)\n\
        Credentials\n  (profile \"default\" not configured \u{2014} run `ant auth login` \
        to set it up)\n";
    let signed_in =
        "Credentials\n  (active) * Profile (user_oauth) [via active_config]  sk-ant-oat01-EXA...\n";
    let say = |ok: bool, text: &str| {
        let text = text.to_string();
        probe_with(&spec, true, &move |_, _| Some((ok, text.clone())))
    };
    assert_eq!(say(true, signed_out), CliAuth::SignedOut);
    assert_eq!(say(true, signed_in), CliAuth::SignedIn);
    assert_eq!(
        say(false, signed_in),
        CliAuth::SignedIn,
        "the exit code is no signal"
    );
    assert_eq!(
        say(
            false,
            "Incorrect Usage: unknown command\nUSAGE:\n   ant auth"
        ),
        CliAuth::Unknown
    );
    assert_eq!(probe_with(&spec, true, &|_, _| None), CliAuth::Unknown);
    assert_eq!(
        probe_with(&spec, false, &|_, _| unreachable!(
            "absent binaries are not run"
        )),
        CliAuth::Absent
    );
}
