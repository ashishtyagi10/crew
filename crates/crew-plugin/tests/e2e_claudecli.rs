//! The Claude Code subscription serving the API-shaped seam: a clean HOME,
//! no `*_API_KEY` anywhere, and a fake `claude` on PATH that answers its
//! own status probe as signed in and its headless mode with the JSON
//! envelope claude 2.x prints. The broker must build the specialist
//! roster on the CLI provider, run a fan-out THROUGH it (Claude Code's
//! tools switched off, the role's system prompt passed), and have
//! `/doctor` say the subscription carries swarm planning too.
mod common;
use common::{messages, run_broker_paced, seed_specialists, unique_dir};

fn send(text: &str) -> String {
    serde_json::json!({"type": "send", "channel": "crew", "text": text}).to_string()
}

/// The fake logs its argv to `claude-run-<pid>.log` — one file per run,
/// because fan legs run in PARALLEL and two appenders interleave a
/// multi-kilobyte prompt — so the test can read the exact headless
/// contract the broker used.
fn write_fake_claude(dir: &std::path::Path) {
    let logs = dir.join("claude-run-");
    let script = format!(
        "#!/bin/sh\n\
         printf '%s\\n' \"$*\" > '{logs}'$$.log\n\
         case \"$*\" in\n\
           'auth status') printf '{{\"loggedIn\": true, \"authMethod\": \"claude.ai\"}}\\n' ;;\n\
           *'--output-format json'*) printf '%s\\n' '{{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"result\":\"cli answer: on the plan\\\\n@done\",\"session_id\":\"s1\",\"total_cost_usd\":0.01,\"usage\":{{\"input_tokens\":7,\"output_tokens\":9}}}}' ;;\n\
           *) printf 'relay answer: on the plan\\n' ;;\n\
         esac\n",
        logs = logs.display()
    );
    let path = dir.join("claude");
    std::fs::write(&path, script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

#[cfg(unix)]
#[test]
fn a_signed_in_claude_code_serves_the_swarm_through_its_cli() {
    let dir = unique_dir("claude-cli-swarm");
    seed_specialists(&dir, &["scout", "sage"]);
    let home = dir.join("home");
    std::fs::create_dir_all(&home).unwrap();
    write_fake_claude(&dir);
    let events = run_broker_paced(
        &dir,
        &[("HOME", home.to_str().unwrap())],
        &[
            (&send("/doctor"), 1500),
            (&send("@scout+sage compare the two approaches"), 4000),
        ],
    );
    let msgs = messages(&events);
    let all: String = msgs.iter().map(|(s, t)| format!("{s}: {t}\n")).collect();

    assert!(
        all.contains("claude-code subscription (via the claude CLI \u{2014} plain replies AND swarm planning)"),
        "{all}"
    );
    // Both specialists answered — and from the JSON (provider) path, not the
    // text (relay) path.
    for who in ["scout", "sage"] {
        assert!(
            msgs.iter()
                .any(|(s, t)| s.starts_with(who) && t.contains("cli answer: on the plan")),
            "{who} did not answer through the CLI provider:\n{all}"
        );
    }
    assert!(
        !all.contains("relay answer"),
        "a fan leg went through the relay:\n{all}"
    );

    // The headless contract, verbatim from what the fake was handed.
    let runs: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().starts_with("claude-run-"))
        .map(|e| std::fs::read_to_string(e.path()).unwrap())
        .collect();
    let log = runs.join("\n----\n");
    let json_runs: Vec<&String> = runs
        .iter()
        .filter(|l| l.contains("--output-format json"))
        .collect();
    assert!(json_runs.len() >= 2, "expected two provider runs:\n{log}");
    for run in &json_runs {
        assert!(run.starts_with("-p "), "{run}");
        assert!(
            run.contains("--tools  --no-session-persistence"),
            "tools not switched off: {run}"
        );
        assert!(run.contains("--model "), "{run}");
        assert!(run.contains("--system-prompt "), "no role prompt: {run}");
    }
    assert!(runs.iter().any(|l| l.trim() == "auth status"), "{log}");
}
