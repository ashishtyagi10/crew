//! What may reach git from a `sys:git` call: the subcommand, the agent's
//! arguments, and the defaults around them. Kept apart from running git
//! (`sysgit`) because this is the half that makes the tool a read — every
//! argument that would make git write a file or run a program stops here.

/// The subcommands that only look. `push`, `commit`, `checkout` and the rest
/// change the repository or the world, and belong to `sys:run`, which asks.
const LOOKS: [&str; 5] = ["status", "diff", "log", "show", "blame"];

/// Why `-c` and `--config` are refused: git's `-c core.pager=…`,
/// `diff.external=…` or `core.fsmonitor=…` is a program to run.
const CONFIG: &str = "it rewrites git's config for the run, and config can name a program to run";

/// Arguments that turn a look into something else, as prefixes (so
/// `--exec-path` and `--config-env` go with their families), each with the
/// reason the agent is given. Git takes none of these abbreviated here —
/// `--outp` is an unknown option to diff, log, show and blame; `status`
/// abbreviates but has none of them — so the full spelling is the only one.
///
/// `-c` is refused for what it means BEFORE a subcommand. After one, where
/// these arguments go, it is only diff's combined format; but an argument
/// whose meaning depends on where git reads it is not one to reason about.
const REFUSED: [(&str, &str); 11] = [
    ("--output", "it writes the result to a file"),
    ("-o", "it names a file to write"),
    ("--exec", "it runs a program"),
    ("--ext-diff", "it runs the diff program git's config names"),
    ("--textconv", "it runs the converter git's config names"),
    ("-c", CONFIG),
    ("--config", CONFIG),
    ("--git-dir", "it points git at another repository"),
    ("--work-tree", "it points git at another working tree"),
    ("--upload-pack", "it names a program to run"),
    ("--show-signature", "it runs gpg to check each signature"),
];

/// The subcommand and the whole command line for `v`, or why it may not run.
pub(super) fn checked(v: &serde_json::Value) -> Result<(&str, Vec<String>), String> {
    let sub = v.get("cmd").and_then(|c| c.as_str()).map(str::trim);
    let sub = sub.ok_or_else(|| "missing string argument \u{201c}cmd\u{201d}".to_string())?;
    if !LOOKS.contains(&sub) {
        return Err(format!(
            "sys:git runs status, diff, log, show and blame \u{2014} \u{201c}{sub}\u{201d} can \
             change the repository, so it goes through sys:run, which asks first"
        ));
    }
    let args = args_of(v)?;
    if let Some((a, why)) = args.iter().find_map(|a| refused(a).map(|w| (a, w))) {
        let a = a.escape_debug();
        return Err(format!("sys:git refuses \u{201c}{a}\u{201d}: {why}"));
    }
    Ok((sub, argv(sub, &args)))
}

/// Why `arg` may not reach git, if it may not. A newline is in no path,
/// revision or option, so an argument carrying one was written to be read as
/// something other than what it looks like.
fn refused(arg: &str) -> Option<&'static str> {
    if arg.contains(['\n', '\r']) {
        return Some("no path, revision or option has a newline in it");
    }
    REFUSED
        .iter()
        .find(|(p, _)| arg.starts_with(p))
        .map(|(_, why)| *why)
}

/// `args` as strings: a list, or one string split on spaces, since a model
/// writes a command line as often as a list. `["-n", 5]` means `-n 5`.
fn args_of(v: &serde_json::Value) -> Result<Vec<String>, String> {
    use serde_json::Value as J;
    match v.get("args") {
        None | Some(J::Null) => Ok(Vec::new()),
        Some(J::String(s)) => Ok(s.split_whitespace().map(str::to_string).collect()),
        Some(J::Array(items)) => items
            .iter()
            .map(|i| match i {
                J::String(s) => Ok(s.clone()),
                J::Number(n) => Ok(n.to_string()),
                other => Err(format!("args are strings, not {other}")),
            })
            .collect(),
        Some(other) => Err(format!("args is a list of strings, not {other}")),
    }
}

/// The command line: git's own switches (no pager, no colour, no fsmonitor
/// hook — a program the config names), the subcommand, its defaults, then the
/// agent's arguments. Defaults go FIRST because git lets the later of two
/// settings win: `-n 20` gives way to the agent's `-n 5`, `--oneline` to its
/// `--format=…`. External diff and text conversion are off even unasked, for
/// the reason their switches are refused.
fn argv(sub: &str, args: &[String]) -> Vec<String> {
    let mut v = vec![
        "--no-pager",
        "-c",
        "color.ui=false",
        "-c",
        "core.fsmonitor=false",
        // `log.showSignature` in a config runs gpg on every commit shown.
        "-c",
        "log.showSignature=false",
    ];
    v.push(sub);
    if sub != "status" {
        v.extend(["--no-ext-diff", "--no-textconv"]);
    }
    match sub {
        "status" => v.extend(["--short", "--branch"]),
        "log" => v.extend(["--oneline", "-n", "20"]),
        // The stat names every file changed, and it survives any cut of the
        // patch below it. Only when the agent has not chosen a shape: given
        // `--stat -p --stat`, git still prints the patch.
        "diff" | "show" if !args.iter().any(|a| shapes(a)) => v.extend(["--stat", "-p"]),
        _ => {}
    }
    let v = v.into_iter().map(str::to_string);
    v.chain(args.iter().cloned()).collect()
}

/// Whether `a` already chooses what a diff's output looks like.
fn shapes(a: &str) -> bool {
    const SHAPES: [&str; 7] = [
        "--stat",
        "--numstat",
        "--shortstat",
        "--dirstat",
        "--name-",
        "--raw",
        "--summary",
    ];
    matches!(a, "-p" | "-u" | "-s" | "--patch" | "--no-patch")
        || SHAPES.iter().any(|p| a.starts_with(p))
}
