//! Against a FAKE `security` binary (a shell script on an isolated path —
//! the same stub-binary seam the auth e2e uses for vendor CLIs): argv is
//! recorded verbatim, secrets land in per-account files, and the macOS
//! keychain itself is never touched by any test.
use super::*;

/// A fake `security`: records its argv, stores/serves/deletes per-account
/// secret files. `variant` tweaks behavior: "fail" exits 1 on everything,
/// "noisy" prints a stderr warning before the secret.
fn fake_security(tag: &str, variant: &str) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!(
        "crew-keychain-{tag}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let script = format!(
        r#"#!/bin/sh
DIR='{dir}'
echo "$@" >> "$DIR/calls.log"
[ '{variant}' = fail ] && exit 1
cmd="$1"; shift
acct=; secret=
while [ $# -gt 0 ]; do
  case "$1" in
    -a) acct="$2"; shift 2;;
    -w) secret="${{2:-}}"; [ $# -gt 1 ] && shift; shift;;
    *) shift;;
  esac
done
case "$cmd" in
  add-generic-password) printf '%s' "$secret" > "$DIR/$acct.secret";;
  find-generic-password)
    [ '{variant}' = noisy ] && echo 'warning: unlocking' >&2
    [ -f "$DIR/$acct.secret" ] || exit 44
    cat "$DIR/$acct.secret"; echo;;
  delete-generic-password) rm -f "$DIR/$acct.secret";;
esac
"#,
        dir = dir.display(),
    );
    let bin = dir.join("security");
    std::fs::write(&bin, script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    (bin, dir)
}

#[test]
fn store_load_delete_round_trip_with_the_documented_argv() {
    let (bin, dir) = fake_security("roundtrip", "ok");
    assert!(store_with(&bin, "dashscope", r#"{"access":"s3cr3t"}"#));
    assert_eq!(
        load_with(&bin, "dashscope").as_deref(),
        Some(r#"{"access":"s3cr3t"}"#)
    );
    delete_with(&bin, "dashscope");
    assert_eq!(load_with(&bin, "dashscope"), None, "deleted item is gone");
    // The exact argv the two calls sent — the documented `security` grammar.
    let calls = std::fs::read_to_string(dir.join("calls.log")).unwrap();
    let lines: Vec<&str> = calls.lines().collect();
    assert_eq!(lines.len(), 4, "{calls}");
    assert_eq!(
        lines[0],
        r#"add-generic-password -U -s crew-oauth -a dashscope -w {"access":"s3cr3t"}"#
    );
    assert_eq!(
        lines[1],
        "find-generic-password -s crew-oauth -a dashscope -w"
    );
    assert_eq!(
        lines[2],
        "delete-generic-password -s crew-oauth -a dashscope"
    );
}

/// `security` prints its password on stdout and warnings on stderr; a merged
/// read would hand back "warning…s3cr3t". Pins the `run_split` design.
#[test]
fn a_stderr_warning_never_corrupts_the_secret() {
    let (bin, _) = fake_security("noisy", "noisy");
    assert!(store_with(&bin, "dashscope", "exact-secret"));
    assert_eq!(
        load_with(&bin, "dashscope").as_deref(),
        Some("exact-secret")
    );
}

#[test]
fn a_refusing_or_absent_binary_answers_false_and_none() {
    let (bin, _) = fake_security("fail", "fail");
    assert!(!store_with(&bin, "dashscope", "s"));
    assert_eq!(load_with(&bin, "dashscope"), None);
    let gone = Path::new("/definitely/not/a/security/binary");
    assert!(!store_with(gone, "dashscope", "s"));
    assert_eq!(load_with(gone, "dashscope"), None);
}

/// The tokens dispatch: keychain first, file only as fallback — and the
/// fallback actually engages when the keychain refuses.
#[test]
fn tokens_dispatch_prefers_the_keychain_and_falls_back_on_refusal() {
    use super::super::tokens::{load_via, store_via, StoredToken};
    let tok = || StoredToken {
        access: "at-kc".into(),
        refresh: None,
        expires_at: 99,
        resource: None,
    };
    // Working keychain: the file must never be written.
    let (bin, dir) = fake_security("dispatch-ok", "ok");
    let file = dir.join("tokens.json");
    store_via(Some(&bin), Some(&file), "dashscope", tok()).unwrap();
    assert!(!file.exists(), "keychain path must not touch the file");
    assert_eq!(
        load_via(Some(&bin), Some(&file), "dashscope")
            .unwrap()
            .access,
        "at-kc"
    );
    // Refusing keychain: the file backend takes over, and a keychain-less
    // load still finds the grant there.
    let (bad, dir2) = fake_security("dispatch-fail", "fail");
    let file2 = dir2.join("tokens.json");
    store_via(Some(&bad), Some(&file2), "dashscope", tok()).unwrap();
    assert!(
        file2.exists(),
        "refused keychain must fall back to the file"
    );
    assert_eq!(
        load_via(Some(&bad), Some(&file2), "dashscope")
            .unwrap()
            .access,
        "at-kc"
    );
    assert_eq!(
        load_via(None, Some(&file2), "dashscope").unwrap().access,
        "at-kc"
    );
}
