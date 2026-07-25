use super::*;
use crew_hive::catalog::{ModelInfo, Vendor};

fn row(slug: &'static str, or_slug: Option<&'static str>, vendor: Vendor) -> ModelInfo {
    ModelInfo {
        name: "n",
        slug,
        or_slug,
        vendor,
        price: None,
        free: false,
        context: 0,
    }
}

#[test]
fn direct_routes_match_the_active_provider() {
    let claude = row(
        "claude-sonnet-5",
        Some("anthropic/claude-sonnet-5"),
        Vendor::Anthropic,
    );
    let qwen = row("qwen-max", Some("qwen/qwen-max"), Vendor::Alibaba);
    assert_eq!(
        route_for(&claude, Some(Provider::Anthropic), true),
        Route::Direct("anthropic")
    );
    assert_eq!(
        route_for(&qwen, Some(Provider::DashScope), true),
        Route::Direct("dashscope")
    );
}

#[test]
fn openrouter_serves_anything_with_an_alias() {
    let claude = row(
        "claude-sonnet-5",
        Some("anthropic/claude-sonnet-5"),
        Vendor::Anthropic,
    );
    let native_only = row("qwen-turbo", None, Vendor::Alibaba);
    assert_eq!(
        route_for(&claude, Some(Provider::OpenRouter), true),
        Route::ViaOpenRouter
    );
    // No alias and OpenRouter can't reach the native endpoint.
    assert!(matches!(
        route_for(&native_only, Some(Provider::OpenRouter), true),
        Route::Missing(_)
    ));
}

#[test]
fn openrouter_routes_a_native_slug_already_in_vendor_slash_model_form() {
    // No separate or_slug, but the native slug is already `vendor/model`
    // — that shape IS an OpenRouter id, so it must route, not be Missing.
    let native_slash = row("mistralai/mixtral-8x7b", None, Vendor::OpenAI);
    assert_eq!(
        route_for(&native_slash, Some(Provider::OpenRouter), true),
        Route::ViaOpenRouter
    );
    assert_eq!(
        Route::ViaOpenRouter.fill_slug(&native_slash),
        "mistralai/mixtral-8x7b"
    );
}

#[test]
fn missing_names_the_key_the_user_would_have_to_set() {
    let gpt = row("gpt-4.1", Some("openai/gpt-4.1"), Vendor::OpenAI);
    assert_eq!(
        route_for(&gpt, Some(Provider::Anthropic), true),
        Route::Missing("OPENROUTER_API_KEY")
    );
}

#[test]
fn no_provider_names_the_first_key_discovery_would_look_for() {
    // Discovery order is DashScope, then OpenRouter, then Anthropic (see
    // `crew_plugin::discover::pick_provider`) — with no provider found at
    // all, the row should point at the first key a user would set, not
    // the last-resort Anthropic one.
    let claude = row(
        "claude-sonnet-5",
        Some("anthropic/claude-sonnet-5"),
        Vendor::Anthropic,
    );
    assert_eq!(
        route_for(&claude, None, true),
        Route::Missing("DASHSCOPE_API_KEY")
    );
}

#[test]
fn unknown_until_the_probe_lands_and_mock_serves_everything() {
    let gpt = row("gpt-4.1", Some("openai/gpt-4.1"), Vendor::OpenAI);
    // Probe not finished: never claim a key is missing on evidence we lack.
    assert_eq!(
        route_for(&gpt, Some(Provider::Anthropic), false),
        Route::Unknown
    );
    assert_eq!(route_for(&gpt, None, false), Route::Unknown);
    assert_eq!(route_for(&gpt, Some(Provider::Mock), true), Route::Mock);
}

#[test]
fn fill_slug_follows_the_route() {
    let claude = row(
        "claude-sonnet-5",
        Some("anthropic/claude-sonnet-5"),
        Vendor::Anthropic,
    );
    assert_eq!(
        Route::ViaOpenRouter.fill_slug(&claude),
        "anthropic/claude-sonnet-5"
    );
    assert_eq!(
        Route::Direct("anthropic").fill_slug(&claude),
        "claude-sonnet-5"
    );
    assert_eq!(Route::Unknown.fill_slug(&claude), "claude-sonnet-5");
}
