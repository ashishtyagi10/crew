//! Which provider would actually serve a catalog model, given the stack the
//! broker will discover. The broker picks exactly ONE provider for every API
//! agent (`crew_plugin::active_provider`), so a pick is only serveable if that
//! provider can route it — OpenRouter reaches everything with an alias, the
//! direct providers only their own vendor. `Unknown` is the honest answer
//! until the key probe lands: we never claim a key is missing on evidence we
//! don't have.
use crew_hive::catalog::{ModelInfo, Vendor};
pub(crate) use crew_plugin::Provider;

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
    /// Rows we know the stack can't serve render dim — in the composer
    /// popup only (`modelpick::model_row` reads this into `MenuItem::dim`;
    /// `cmdmenu::menu_cells` honours it). The input-bar picker
    /// (`suggestvalues::options_for`) flattens rows to bare `(value, desc)` pairs
    /// and never sees this field.
    pub(crate) fn unserveable(&self) -> bool {
        matches!(self, Self::Missing(_))
    }
}

/// Resolve the route for one catalog row. `probed` is whether the login-shell
/// key probe has completed; before it has, everything is `Unknown`.
pub(crate) fn route_for(m: &ModelInfo, provider: Option<Provider>, probed: bool) -> Route {
    let Some(provider) = provider else {
        // Discovery order is DashScope, then OpenRouter, then Anthropic — name
        // the first key a user would set, not the last-resort one.
        return if probed {
            Route::Missing("DASHSCOPE_API_KEY")
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
#[path = "modelroute_tests.rs"]
mod tests;
