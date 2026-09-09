//! The CLI-minted rung end to end: a clean HOME, no `*_API_KEY` anywhere,
//! and a fake `ant` on PATH that answers its two documented commands the
//! way ant 1.31.0 does — `auth status` naming a `user_oauth` profile, and
//! `auth print-credentials --access-token` printing one bare token. The
//! broker must list anthropic as signed in, route a plain message through
//! the native Anthropic provider, and present the minted token as
//! `Authorization: Bearer` + the OAuth beta header (never `x-api-key`) to a
//! loopback stub standing in for `api.anthropic.com`. Then the secrets
//! sweep: the token reached the stub's socket and nothing else.
mod common;
use common::oauthstub::{serve, sweep};
use common::{messages, run_broker_paced, seed_specialists, unique_dir};

const TOKEN: &str = "sk-ant-oat01-e2e-minted-token-4242";

fn send(text: &str) -> String {
    serde_json::json!({"type": "send", "channel": "crew", "text": text}).to_string()
}

/// The fake CLI reads the token from its environment, so no file in the
/// test dir ever holds it — the sweep can then treat any hit as a leak.
fn write_fake_ant(dir: &std::path::Path) {
    let script = "#!/bin/sh\n\
        case \"$*\" in\n\
          'auth status') printf 'Active profile:  default\\nCredentials\\n  (active) * Profile (user_oauth) [via active_config]  sk-ant-oat01-EXA...\\n' ;;\n\
          'auth print-credentials --access-token') printf '%s\\n' \"$FAKE_ANT_TOKEN\" ;;\n\
          *) echo \"Incorrect Usage: unknown command\" >&2; exit 2 ;;\n\
        esac\n";
    let path = dir.join("ant");
    std::fs::write(&path, script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

/// An Anthropic-wire reply: text blocks + usage, whatever the prompt.
fn anthropic_reply(text: &str) -> String {
    serde_json::json!({
        "type": "message",
        "content": [{"type": "text", "text": text}],
        "usage": {"input_tokens": 10, "output_tokens": 10}
    })
    .to_string()
}

#[cfg(unix)]
#[test]
fn a_signed_in_ant_profile_serves_with_a_minted_bearer() {
    let dir = unique_dir("console-oauth");
    seed_specialists(&dir, &["scout"]);
    let home = dir.join("home");
    std::fs::create_dir_all(&home).unwrap();
    write_fake_ant(&dir);
    let stub = serve(|path, body| match path {
        "/v1/messages" if body.contains("You route a user's message") => {
            (200, anthropic_reply("SHAPE: reply"))
        }
        "/v1/messages" => (
            200,
            anthropic_reply("stub answer: minted and served\n@done"),
        ),
        other => (404, format!(r#"{{"error":"no route {other}"}}"#)),
    });
    let events = run_broker_paced(
        &dir,
        &[
            ("HOME", home.to_str().unwrap()),
            ("FAKE_ANT_TOKEN", TOKEN),
            ("ANTHROPIC_BASE_URL", &stub.base),
        ],
        &[
            (&send("/model"), 800),
            (&send("/login"), 800),
            (&send("say hello please"), 4000),
        ],
    );
    let msgs = messages(&events);
    let all: String = msgs.iter().map(|(s, t)| format!("{s}: {t}\n")).collect();

    // The picker and the front door both read the CLI's verdict.
    assert!(
        all.contains("1. anthropic \u{2014} signed in \u{00b7} OAuth profile via its CLI"),
        "{all}"
    );
    assert!(
        all.contains("\u{25cb} anthropic \u{2014} \u{2713} signed in (vendor CLI)"),
        "{all}"
    );
    // A plain message answered THROUGH the minted bearer.
    assert!(all.contains("stub answer: minted and served"), "{all}");

    // Every call the stub saw carried the bearer + beta header, never a key.
    let heads = stub.heads.lock().unwrap().clone();
    assert!(!heads.is_empty(), "the stub saw no calls:\n{all}");
    for head in &heads {
        let low = head.to_ascii_lowercase();
        assert!(
            low.contains(&format!("authorization: bearer {TOKEN}")),
            "no bearer in:\n{head}"
        );
        assert!(low.contains("anthropic-beta: oauth-2025-04-20"), "{head}");
        assert!(
            !low.contains("x-api-key"),
            "a key header rode along:\n{head}"
        );
    }

    // The token never landed anywhere but the socket: no file, no stdout,
    // no stderr, no session log.
    let stderr = std::fs::read_to_string(dir.join("broker-stderr.log")).unwrap_or_default();
    let hits = sweep(&dir, &[("stdout", &all), ("stderr", &stderr)], TOKEN, &[]);
    assert_eq!(hits, Vec::<String>::new(), "token material leaked");
}

/// Without a profile the same machine is key-less: the CLI is installed
/// but signed out, so `/login` and `/model` both say exactly what to run
/// (grayed, unnumbered), and `/logout anthropic` hands the sign-out to the
/// CLI instead of deleting anything.
#[cfg(unix)]
#[test]
fn a_signed_out_ant_is_the_sign_in_affordance() {
    let dir = unique_dir("console-signed-out");
    let home = dir.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let script = "#!/bin/sh\n\
        case \"$*\" in\n\
          'auth status') printf 'Credentials\\n  (profile \"default\" not configured \\xe2\\x80\\x94 run `ant auth login` to set it up)\\n' ;;\n\
          *) echo 'not logged in (profile \"default\")' >&2; exit 1 ;;\n\
        esac\n";
    let path = dir.join("ant");
    std::fs::write(&path, script).unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let events = run_broker_paced(
        &dir,
        &[("HOME", home.to_str().unwrap())],
        &[
            (&send("/login"), 800),
            (&send("/logout anthropic"), 500),
            (&send("/model"), 500),
        ],
    );
    let all: String = messages(&events)
        .iter()
        .map(|(s, t)| format!("{s}: {t}\n"))
        .collect();
    assert!(
        all.contains("\u{25cb} anthropic \u{2014} signed out \u{00b7} run `ant auth login`"),
        "{all}"
    );
    assert!(
        all.contains("anthropic signs out through its own CLI \u{2014} run `ant auth logout`"),
        "{all}"
    );
    // The picker's affordance, grayed and unnumbered: nothing to pick, the
    // command to run.
    assert!(
        all.contains(
            "\u{25cb} anthropic \u{2014} signed out \u{00b7} sign in: run `ant auth login`"
        ),
        "{all}"
    );
    assert!(!all.contains("anthropic \u{2014} signed in"), "{all}");
}
