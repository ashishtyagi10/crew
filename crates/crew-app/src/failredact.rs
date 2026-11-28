//! Secrets masked out of a command line before it goes to a model.
//!
//! A span's line is the argv from the process table, so the SHELL has already
//! expanded it: `--token=$KEY` is the real token by the time crew reads it.
//! `??` and the smith prefill send that line off the machine, so each secret
//! a command line commonly carries becomes `***` and the rest is kept — the
//! model still needs the program, the subcommand and the flags to help.
use std::sync::LazyLock;

use regex::{Captures, Regex};

/// A value: quoted (spaces allowed) or a bare word.
const VALUE: &str = r#""[^"]*"|'[^']*'|\S+"#;

static USERINFO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"://[^\s/@]+@").unwrap());
static AUTH_HEADER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(authorization:\s*)(?:(?:bearer|basic|token)\s+)?[^"'\s]+"#).unwrap()
});
static BEARER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)\b(bearer\s+)[^"'\s]+"#).unwrap());
static FLAG: LazyLock<Regex> = LazyLock::new(|| {
    let name = r"--[\w-]*(?i:token|key|secret|password|passwd|auth|credential)[\w-]*";
    Regex::new(&format!("({name})(=|\\s+)({VALUE})")).unwrap()
});
static ENV: LazyLock<Regex> = LazyLock::new(|| {
    let name = r"\b[A-Z0-9_]*(?:KEY|TOKEN|SECRET|PASSWORD)[A-Z0-9_]*";
    Regex::new(&format!("({name}=)({VALUE})")).unwrap()
});

/// `line` with URL passwords, auth headers, bearer tokens, secret-named
/// flags and secret-named environment assignments masked as `***`.
pub(crate) fn redact(line: &str) -> String {
    let s = USERINFO.replace_all(line, "://***@");
    let s = AUTH_HEADER.replace_all(&s, "${1}***");
    let s = BEARER.replace_all(&s, "${1}***");
    // `--auth --verbose`: a flag taking no value must not eat the next flag.
    let s = FLAG.replace_all(&s, |c: &Captures| match (&c[2], &c[3]) {
        (sep, v) if sep != "=" && v.starts_with('-') => c[0].to_string(),
        (sep, _) => format!("{}{sep}***", &c[1]),
    });
    ENV.replace_all(&s, "${1}***").into_owned()
}

#[cfg(test)]
#[path = "failredact_tests.rs"]
mod tests;
