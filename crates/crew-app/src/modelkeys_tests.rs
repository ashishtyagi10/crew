use super::*;

/// A throwaway executable shell script at `d/name`, `chmod +x`'d.
fn script(d: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
    let p = d.join(name);
    std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    p
}

#[test]
fn bounded_shell_env_captures_a_quick_probe() {
    let dir = tempfile::tempdir().unwrap();
    let sh = script(dir.path(), "sh", "printf 'ANTHROPIC_API_KEY=sk-ant-1\\n'");
    let out = bounded_shell_env(sh.to_str().unwrap(), Duration::from_secs(3))
        .expect("a fast script must produce output before the deadline");
    assert!(out.contains("ANTHROPIC_API_KEY=sk-ant-1"));
}

#[test]
fn bounded_shell_env_kills_a_hanging_probe_within_the_timeout() {
    // A blocking rc file (stray prompt, hung network mount) must not park
    // this thread for the process lifetime — `Command::output()` alone has
    // no deadline and would wait on this forever.
    let dir = tempfile::tempdir().unwrap();
    let sh = script(dir.path(), "sh", "sleep 5");
    let start = Instant::now();
    let out = bounded_shell_env(sh.to_str().unwrap(), Duration::from_millis(150));
    let elapsed = start.elapsed();
    assert!(out.is_none(), "a hung shell must produce no env to merge");
    assert!(
        elapsed < Duration::from_secs(2),
        "must be killed near the 150ms bound, not wait out the 5s sleep: {elapsed:?}"
    );
}

#[test]
fn shell_probed_pin_is_honoured() {
    let keys: HashSet<String> = ["OPENROUTER_API_KEY".to_string()].into_iter().collect();
    assert_eq!(
        resolve(&keys, Some("openrouter")),
        Some(crew_plugin::Provider::OpenRouter)
    );
}

#[test]
fn pin_absent_from_process_env_but_present_in_probe_is_still_honoured() {
    // Simulates a GUI-launched process: this process never saw
    // CREW_PROVIDER, but the login-shell probe found it in an rc file.
    let keys: HashSet<String> = ["ANTHROPIC_API_KEY".to_string()].into_iter().collect();
    // Without the pin, auto-discovery lands on the only key present.
    assert_eq!(resolve(&keys, None), Some(crew_plugin::Provider::Anthropic));
    // With the probed pin honoured, the explicit forced provider wins
    // instead — even though no OPENROUTER_API_KEY is in the key set.
    assert_eq!(
        resolve(&keys, Some("openrouter")),
        Some(crew_plugin::Provider::OpenRouter)
    );
}

#[test]
fn key_order_auto_discovery_applies_when_there_is_no_pin() {
    let keys: HashSet<String> = ["DASHSCOPE_API_KEY".to_string()].into_iter().collect();
    assert_eq!(resolve(&keys, None), Some(crew_plugin::Provider::DashScope));
    assert_eq!(resolve(&HashSet::new(), None), None);
}

#[test]
fn merge_process_pin_wins_over_shell_pin() {
    // Process already has CREW_PROVIDER=anthropic; shell output has
    // CREW_PROVIDER=openrouter. The process value must win.
    let mut probed = Probed {
        keys: HashSet::new(),
        provider_pin: Some("anthropic".to_string()),
        openrouter_key: None,
    };
    let shell_env = "CREW_PROVIDER=openrouter\nOPENROUTER_API_KEY=sk-or-123\n";
    merge_shell_env(&mut probed, shell_env);
    assert_eq!(
        probed.provider_pin,
        Some("anthropic".to_string()),
        "process pin must not be overwritten"
    );
    // The key should be added though.
    assert!(probed.keys.contains("OPENROUTER_API_KEY"));
}

#[test]
fn merge_adopts_shell_pin_when_process_has_none() {
    // Process has no CREW_PROVIDER; shell has CREW_PROVIDER=openrouter.
    // The shell value must be adopted.
    let mut probed = Probed {
        keys: HashSet::new(),
        provider_pin: None,
        openrouter_key: None,
    };
    let shell_env = "CREW_PROVIDER=openrouter\nOPENROUTER_API_KEY=sk-or-123\n";
    merge_shell_env(&mut probed, shell_env);
    assert_eq!(
        probed.provider_pin,
        Some("openrouter".to_string()),
        "shell pin must be adopted when process has none"
    );
}

#[test]
fn merge_process_openrouter_key_wins_over_shell_key() {
    // Process already has a value (e.g. this test process's own env);
    // shell output has a different one. The process value must win — same
    // precedence as `CREW_PROVIDER` above, per the broker's shellenv rule.
    let mut probed = Probed {
        keys: HashSet::new(),
        provider_pin: None,
        openrouter_key: Some("sk-or-process".to_string()),
    };
    let shell_env = "OPENROUTER_API_KEY=sk-or-shell\n";
    merge_shell_env(&mut probed, shell_env);
    assert_eq!(
        probed.openrouter_key,
        Some("sk-or-process".to_string()),
        "process key must not be overwritten by the shell-probed value"
    );
}

#[test]
fn merge_adopts_shell_openrouter_key_when_process_has_none() {
    // Process never saw OPENROUTER_API_KEY (the Finder-launch case this
    // probe exists for); shell has it. The shell value must be adopted so
    // `crate::modelfetch::spawn` can actually see a key.
    let mut probed = Probed {
        keys: HashSet::new(),
        provider_pin: None,
        openrouter_key: None,
    };
    let shell_env = "OPENROUTER_API_KEY=sk-or-shell\n";
    merge_shell_env(&mut probed, shell_env);
    assert_eq!(
        probed.openrouter_key,
        Some("sk-or-shell".to_string()),
        "shell key must be adopted when the process has none"
    );
}

#[test]
fn merge_ignores_empty_values() {
    // Shell has CREW_PROVIDER with an empty value; should not record it.
    let mut probed = Probed {
        keys: HashSet::new(),
        provider_pin: None,
        openrouter_key: None,
    };
    let shell_env = "CREW_PROVIDER=\nDASHSCOPE_API_KEY=sk-aak-123\n";
    merge_shell_env(&mut probed, shell_env);
    assert_eq!(
        probed.provider_pin, None,
        "empty shell value must not overwrite process None"
    );
    // Non-empty values are still recorded.
    assert!(probed.keys.contains("DASHSCOPE_API_KEY"));
}

#[test]
fn merge_ignores_keys_outside_the_interesting_set() {
    // Shell has HOME and UNKNOWN_VAR; should ignore them.
    let mut probed = Probed {
        keys: HashSet::new(),
        provider_pin: None,
        openrouter_key: None,
    };
    let shell_env = "HOME=/Users/ashish\nUNKNOWN_VAR=value\nANTHROPIC_API_KEY=sk-ant-123\n";
    merge_shell_env(&mut probed, shell_env);
    assert!(
        !probed.keys.contains("HOME"),
        "HOME must not be recorded (not in KEYS)"
    );
    assert!(
        !probed.keys.contains("UNKNOWN_VAR"),
        "unknown vars must not be recorded"
    );
    assert!(probed.keys.contains("ANTHROPIC_API_KEY"));
}
