//! Which provider would actually serve a catalog model, given the stack the
//! broker will discover. The broker picks exactly ONE provider for every API
//! agent (`crew_plugin::active_provider`), so a pick is only serveable if that
//! provider can route it — OpenRouter reaches everything with an alias, the
//! direct providers only their own vendor. `Unknown` is the honest answer
//! until the key probe lands: we never claim a key is missing on evidence we
//! don't have.
use crew_hive::catalog::{ModelInfo, Vendor};
pub(crate) use crew_plugin::Provider;

// Data layer for the /model picker (a later task in this series); nothing
// renders it yet, so every item below is dead by clippy's count until then.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Route {
    /// Served straight by the named provider ("anthropic", "dashscope").
    Direct(&'static str),
    /// Served through OpenRouter, using the row's `or_slug`.
    ViaOpenRouter,
    /// Mock provider (tests / `CREW_BROKER_MOCK_REPLY`): everything "works".
    Mock,
    /// The key probe hasn't finished — don't dim, don't promise.
    Unknown,
    /// Not serveable by the active stack; names the key that would fix it.
    Missing(&'static str),
}

#[allow(dead_code)]
impl Route {
    /// The slug to send: the OpenRouter alias when OpenRouter serves it, the
    /// native slug otherwise.
    pub(crate) fn fill_slug(&self, m: &ModelInfo) -> String {
        match self {
            Self::ViaOpenRouter => m.or_slug.unwrap_or(m.slug).to_string(),
            _ => m.slug.to_string(),
        }
    }
    /// Dim hint fragment for the row's desc column ("" when it adds nothing).
    pub(crate) fn hint(&self) -> String {
        match self {
            Self::Direct(p) => (*p).to_string(),
            Self::ViaOpenRouter => "via openrouter".to_string(),
            Self::Mock => "mock".to_string(),
            Self::Unknown => String::new(),
            Self::Missing(k) => format!("needs {k}"),
        }
    }
    /// Rows we know the stack can't serve render dim.
    pub(crate) fn unserveable(&self) -> bool {
        matches!(self, Self::Missing(_))
    }
}

/// Resolve the route for one catalog row. `probed` is whether the login-shell
/// key probe has completed; before it has, everything is `Unknown`.
#[allow(dead_code)]
pub(crate) fn route_for(m: &ModelInfo, provider: Option<Provider>, probed: bool) -> Route {
    let Some(provider) = provider else {
        return if probed {
            Route::Missing("ANTHROPIC_API_KEY")
        } else {
            Route::Unknown
        };
    };
    if !probed {
        return Route::Unknown;
    }
    match provider {
        Provider::Mock => Route::Mock,
        Provider::Anthropic if m.vendor == Vendor::Anthropic => Route::Direct("anthropic"),
        Provider::DashScope if m.vendor == Vendor::Alibaba => Route::Direct("dashscope"),
        // A native slug already in `vendor/model` form IS an OpenRouter id
        // (OpenRouter routes by that shape), even with no separate `or_slug`.
        Provider::OpenRouter if m.or_slug.is_some() || m.slug.contains('/') => Route::ViaOpenRouter,
        Provider::OpenRouter => Route::Missing("a model OpenRouter serves"),
        _ => Route::Missing("OPENROUTER_API_KEY"),
    }
}

#[cfg(test)]
mod tests {
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
}
