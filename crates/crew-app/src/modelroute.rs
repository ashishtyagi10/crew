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
    /// The environment variable this row is blocked on, when that is a key
    /// crew can actually store. `Missing` also carries human phrases — see
    /// `route_for`'s `Missing("a model OpenRouter serves")` — which name no
    /// variable and must not produce a key prompt for a nonsense name.
    pub(crate) fn needs_key(&self) -> Option<&'static str> {
        match self {
            Self::Missing(k) => crew_plugin::credentials::VARS.contains(k).then_some(*k),
            _ => None,
        }
    }
}

/// Resolve the route for one catalog row. `probed` is whether the login-shell
/// key probe has completed; before it has, everything is `Unknown`.
/// [`route_for`] plus what the user actually holds. A row whose vendor crew
/// can reach with a key the user ALREADY has is serveable — picking it
/// switches the active provider (the broker does that in `/model`), and the
/// old behaviour of asking for a key they had was the papercut six providers
/// made likely.
pub(crate) fn route_with_keys(
    m: &ModelInfo,
    provider: Option<Provider>,
    probed: bool,
    has_key: impl Fn(&str) -> bool,
) -> Route {
    let base = route_for(m, provider, probed);
    let Route::Missing(var) = base else {
        return base;
    };
    // Only a real variable can be held; `Missing` also carries prose.
    if !crew_plugin::credentials::VARS.contains(&var) || !has_key(var) {
        return base;
    }
    match crew_plugin::credentials::provider_for(var) {
        Some(name) => Route::Direct(name),
        None => base,
    }
}

pub(crate) fn route_for(m: &ModelInfo, provider: Option<Provider>, probed: bool) -> Route {
    let Some(provider) = provider else {
        // Name the key THIS row needs, not the first one discovery happens to
        // look for. On a keyless machine — the case this whole feature exists
        // for — every row used to ask for `DASHSCOPE_API_KEY`, so a user
        // holding an Anthropic key had no route to enter it and pasting it at
        // the Claude row would have stored it under Alibaba's variable and
        // pinned `dashscope`, authenticating against the wrong endpoint.
        return if probed {
            Route::Missing(vendor_key(m.vendor))
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
        // A table provider serves its own vendor natively — that is the whole
        // point of it existing, and why an OpenAI row no longer asks for an
        // OpenRouter key when the user holds an OpenAI one.
        Provider::Direct(d) if m.vendor == d.vendor => Route::Direct(d.name),
        // A native slug already in `vendor/model` form IS an OpenRouter id
        // (OpenRouter routes by that shape), even with no separate `or_slug`.
        Provider::OpenRouter if m.or_slug.is_some() || m.slug.contains('/') => Route::ViaOpenRouter,
        Provider::OpenRouter => Route::Missing("a model OpenRouter serves"),
        // The active provider cannot serve this row, so name the key that
        // WOULD — the row's own vendor. This used to answer
        // `OPENROUTER_API_KEY` for every such row regardless of vendor, which
        // was harmless while there were three providers and one of them
        // routed everything, and became wrong as soon as a user could hold an
        // Anthropic key while OpenAI was active.
        _ => Route::Missing(vendor_key(m.vendor)),
    }
}

/// The key that would make a row of this vendor serveable when NO provider is
/// configured at all: the vendor's own, when crew can reach that vendor
/// directly (Anthropic, Alibaba/DashScope, and every row of the provider
/// table), and `OPENROUTER_API_KEY` for the vendors OpenRouter really is the
/// only route to — including `Vendor::OpenRouter` itself.
///
/// This is the ONLY producer of a key name in the no-provider case, and every
/// name it returns is in `credentials::VARS`, so `needs_key` accepts all three
/// and the prompt can actually ask for each.
fn vendor_key(v: Vendor) -> &'static str {
    match v {
        Vendor::Anthropic => "ANTHROPIC_API_KEY",
        Vendor::Alibaba => "DASHSCOPE_API_KEY",
        // Ask for the vendor's OWN key when crew can talk to that vendor
        // directly; OpenRouter remains the honest answer only for vendors it
        // is genuinely the only route to.
        other => crew_plugin::DIRECT
            .iter()
            .find(|d| d.vendor == other)
            .map_or("OPENROUTER_API_KEY", |d| d.var),
    }
}

#[cfg(test)]
#[path = "modelroute_tests.rs"]
mod tests;
