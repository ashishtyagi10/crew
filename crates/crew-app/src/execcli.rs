//! `crew exec "<prompt>"` — one headless agent run, for scripts, CI and cron (the shape of
//! `claude -p` and `codex exec`). The answer goes to stdout and progress to stderr, so
//! `crew exec … > out.txt` captures only the answer.
//!
//! Exit codes: 0 task finished ok; 1 task failed, errored or the broker died; 2 usage error
//! (no prompt); 3 the run needed an approval and none was granted (re-run with `--yes`).
use crate::execevent::{EXIT_OK, EXIT_USAGE};
use std::io::{IsTerminal, Read};

const USAGE: &str =
    "usage: crew exec [--yes] \"<prompt>\"   (or pipe the prompt: echo … | crew exec)\n\
  --yes, --allow-edits   allow the approvals the run asks for (default: refuse them)\n\
exit: 0 ok, 1 failed, 2 usage, 3 an approval was refused";

/// What the command line asked for.
#[derive(Debug, PartialEq)]
pub(crate) struct Opts {
    pub prompt: String,
    pub allow: bool,
}

/// Parse the words after `exec`. `stdin` is read only when no prompt was given or it is `-`,
/// and is a closure so a test (and a terminal, which must not hang) can stand in for it.
pub(crate) fn parse(args: &[String], stdin: impl FnOnce() -> String) -> Result<Opts, String> {
    let (mut allow, mut words, mut flags) = (false, Vec::new(), true);
    for a in args {
        match a.as_str() {
            "--" if flags => flags = false,
            "--yes" | "--allow-edits" if flags => allow = true,
            s if flags && s.len() > 1 && s.starts_with('-') => {
                return Err(format!("unknown option {s}"))
            }
            s => words.push(s),
        }
    }
    let prompt = if words.is_empty() || words == ["-"] {
        stdin()
    } else {
        words.join(" ")
    };
    match prompt.trim() {
        "" => Err("no prompt given".to_string()),
        p => Ok(Opts {
            prompt: p.to_string(),
            allow,
        }),
    }
}

/// `crew exec …`. `None` when this is not an exec invocation.
pub(crate) fn dispatch_cli() -> Option<i32> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("exec") {
        return None;
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        return Some(EXIT_OK);
    }
    // A terminal on stdin means nobody is piping a prompt: say so instead of waiting forever.
    let read = || {
        let mut s = String::new();
        if !std::io::stdin().is_terminal() {
            let _ = std::io::stdin().read_to_string(&mut s);
        }
        s
    };
    Some(match parse(&args[1..], read) {
        Ok(o) => crate::execrun::run(&o),
        Err(e) => {
            eprintln!("crew exec: {e}\n{USAGE}");
            EXIT_USAGE
        }
    })
}

#[cfg(test)]
#[path = "execcli_tests.rs"]
mod tests;
