//! The subscription rung of the route: a signed-in Claude Code serving
//! Claude rows. Its own file because `modelroute_tests.rs` is over the
//! line cap and may not grow.
use super::*;

fn row(slug: &'static str, or_slug: Option<&'static str>, vendor: Vendor) -> ModelInfo {
    ModelInfo {
        name: slug,
        slug,
        or_slug,
        vendor,
        price: None,
        free: false,
        context: 0,
    }
}

/// A signed-in Claude Code serves every Claude row — with no Anthropic key
/// at all, and whoever is active: picking the row moves the pin there. A
/// sign-in for anyone else changes nothing, and a held key still wins the
/// native route.
#[test]
fn a_claude_code_sign_in_makes_claude_rows_serveable() {
    let claude = row(
        "claude-sonnet-5",
        Some("anthropic/claude-sonnet-5"),
        Vendor::Anthropic,
    );
    let sub = |n: &str| n == "claude-code";
    let route = route_with_signins(&claude, Some(Provider::DashScope), true, |_| false, sub);
    assert_eq!(route, Route::Direct("claude-code"));
    assert!(!route.unserveable() && route.needs_key().is_none());
    assert_eq!(
        route_with_signins(
            &claude,
            Some(Provider::DashScope),
            true,
            |_| false,
            |n| n == "codex"
        ),
        Route::Missing("ANTHROPIC_API_KEY")
    );
    assert_eq!(
        route_with_signins(
            &claude,
            Some(Provider::DashScope),
            true,
            |v| v == "ANTHROPIC_API_KEY",
            sub
        ),
        Route::Direct("anthropic"),
        "a held key keeps the native route"
    );
    // Once pinned, Claude Code IS the active provider and routes directly.
    assert_eq!(
        route_for(&claude, Some(Provider::ClaudeCli), true),
        Route::Direct("claude-code")
    );
    let gpt = row("gpt-4.1", Some("openai/gpt-4.1"), Vendor::OpenAI);
    assert_eq!(
        route_for(&gpt, Some(Provider::ClaudeCli), true),
        Route::Missing("OPENAI_API_KEY"),
        "the CLI serves Claude only"
    );
}
