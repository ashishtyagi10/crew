//! The sidecar is opt-in in every direction, and these are the directions: unset, set to
//! something that is not there, and set to something that is. Then the tier the swarm
//! serves on: Standard by default, Cheap only when asked.
use super::*;
use crate::broker::testenv;

/// `CREW_SIDECAR` is read by nothing else, so setting it here races with no other test.
fn with_sidecar<T>(value: Option<&str>, f: impl FnOnce() -> T) -> T {
    match value {
        Some(v) => std::env::set_var("CREW_SIDECAR", v),
        None => std::env::remove_var("CREW_SIDECAR"),
    }
    let out = f();
    std::env::remove_var("CREW_SIDECAR");
    out
}

#[test]
fn no_sidecar_is_configured_by_default() {
    // The whole non-negotiable in one assertion: crew is a single binary with no runtime
    // dependency on anything, and nothing here changes that until somebody asks.
    assert_eq!(with_sidecar(None, sidecar_command), None);
}

#[test]
fn a_sidecar_that_is_not_installed_is_not_used() {
    // A machine with no Python must go on working exactly as it did — the goal's condition.
    assert_eq!(
        with_sidecar(Some("crew-no-such-engine --serve"), sidecar_command),
        None
    );
}

#[test]
fn a_sidecar_that_is_installed_is_read_as_a_command_and_its_arguments() {
    let program = if cfg!(windows) { "cmd" } else { "sh" };
    let line = format!("{program} -c true");
    let got = with_sidecar(Some(&line), sidecar_command);
    assert_eq!(
        got,
        Some((program.to_string(), vec!["-c".into(), "true".into()]))
    );
}

#[test]
fn a_blank_setting_is_no_sidecar_rather_than_an_empty_command() {
    assert_eq!(with_sidecar(Some("   "), sidecar_command), None);
}

/// A store holding only an Anthropic key and its pin — the one provider whose model id
/// is the tier's, so the resolved model shows which tier the backend asked for.
fn anthropic_store(tag: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("crew-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let store = dir.join("credentials.json");
    crate::credentials::save_key_at(
        &store,
        "ANTHROPIC_API_KEY",
        "sk-ant-fake",
        Some("anthropic"),
    )
    .unwrap();
    (dir, store)
}

/// The swarm serves at Standard by default — the relay roster's tier, so the footer and
/// the swarm name the same model — and `CREW_SWARM_TIER=cheap` is the one way down.
/// The variable is set inside the guard's lock, never around it: every env write in this
/// binary serialises on `testenv::LOCK`.
#[test]
fn the_backend_serves_at_standard_by_default_and_at_cheap_only_when_asked() {
    let (dir, store) = anthropic_store("swarm-tier");
    let env = testenv::no_provider_with_store(&store);
    std::env::remove_var("CREW_SWARM_TIER");
    let (_, _, budget, model, replan) = backend(None);
    assert_eq!(model, ModelTier::Standard.model_id(), "the default tier");
    assert!(
        budget.is_some() && replan.is_some(),
        "a real-provider backend"
    );

    std::env::set_var("CREW_SWARM_TIER", "cheap");
    let (_, _, _, model, _) = backend(None);
    std::env::remove_var("CREW_SWARM_TIER");
    assert_eq!(model, ModelTier::Cheap.model_id(), "the escape hatch");
    drop(env);
    let _ = std::fs::remove_dir_all(&dir);
}
