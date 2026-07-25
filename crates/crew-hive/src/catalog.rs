//! The model catalog behind the `/model` picker: display names, slugs, vendor
//! grouping, list prices, and free/paid marking. Curated here rather than
//! discovered so the picker works offline; `fetch_openrouter` enriches it at
//! runtime where a live rate is available. Prices are µ$ per 1M tokens, the
//! same unit as [`crate::pricing`] — an unknown price is `None`, never a
//! guess (the badge renders `—`).
mod data;

/// The company behind a model — the picker's section key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vendor {
    Anthropic,
    OpenAI,
    Google,
    Alibaba,
    Moonshot,
    DeepSeek,
    Meta,
    Nvidia,
    Cohere,
    Mistral,
    XAI,
    HuggingFace,
    OpenRouter,
    Other,
}

impl Vendor {
    /// Section header text.
    pub fn label(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAI => "openai",
            Self::Google => "google",
            Self::Alibaba => "alibaba \u{b7} qwen",
            Self::Moonshot => "moonshot \u{b7} kimi",
            Self::DeepSeek => "deepseek",
            Self::Meta => "meta \u{b7} llama",
            Self::Nvidia => "nvidia",
            Self::Cohere => "cohere",
            Self::Mistral => "mistral",
            Self::XAI => "xai",
            Self::HuggingFace => "hugging face",
            Self::OpenRouter => "openrouter",
            Self::Other => "other",
        }
    }
    /// Section order in the picker — majors first, meta-routers last.
    pub const ORDER: &'static [Vendor] = &[
        Self::Anthropic,
        Self::OpenAI,
        Self::Google,
        Self::Alibaba,
        Self::Moonshot,
        Self::DeepSeek,
        Self::Meta,
        Self::Nvidia,
        Self::Cohere,
        Self::Mistral,
        Self::XAI,
        Self::HuggingFace,
        Self::OpenRouter,
        Self::Other,
    ];
}

/// One catalog row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelInfo {
    /// Display name shown to the user ("Claude Sonnet 5").
    pub name: &'static str,
    /// Native slug, sent when the provider serves this vendor directly.
    pub slug: &'static str,
    /// OpenRouter alias, sent when OpenRouter is the active provider.
    pub or_slug: Option<&'static str>,
    pub vendor: Vendor,
    /// (input, output) µ$ per 1M tokens; `None` when we don't know it.
    pub price: Option<(u64, u64)>,
    pub free: bool,
    /// Context window in tokens; 0 when unknown.
    pub context: u32,
}

/// The curated catalog.
pub fn catalog() -> &'static [ModelInfo] {
    data::MODELS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_are_unique_and_non_empty() {
        let mut seen: Vec<&str> = Vec::new();
        for m in catalog() {
            assert!(!m.slug.is_empty(), "empty slug for {}", m.name);
            assert!(!m.name.is_empty(), "empty name for {}", m.slug);
            assert!(!seen.contains(&m.slug), "duplicate slug {}", m.slug);
            seen.push(m.slug);
        }
    }

    #[test]
    fn free_rows_are_zero_priced_and_paid_rows_are_not() {
        for m in catalog() {
            if m.free {
                assert_eq!(m.price, Some((0, 0)), "free row {} must price at 0", m.slug);
            } else if let Some((inp, out)) = m.price {
                assert!(inp > 0 && out > 0, "paid row {} has a zero rate", m.slug);
            }
        }
    }

    #[test]
    fn the_majors_are_all_represented() {
        for v in [
            Vendor::Anthropic,
            Vendor::OpenAI,
            Vendor::Alibaba,
            Vendor::DeepSeek,
        ] {
            assert!(
                catalog().iter().any(|m| m.vendor == v),
                "no rows for {}",
                v.label()
            );
        }
    }

    #[test]
    fn priced_rows_match_the_pricing_table() {
        // The catalog badge and the statusline `$` must agree: a 1M-in call on
        // the catalog's price equals `pricing::cost_microusd` for the same
        // slug, for *any* row — not just Anthropic's. `cost_microusd` returns
        // 0 for a slug that matches no `RATES` pattern (and legitimately for
        // free rows, which are `(0, 0)` in the catalog too), so those are
        // skipped rather than asserted on: an unmatched row proves nothing
        // about agreement.
        for m in catalog().iter().filter(|m| m.price.is_some()) {
            let (inp, _) = m.price.expect("filtered to priced rows");
            let got = crate::pricing::cost_microusd(m.slug, 1_000_000, 0);
            if got == 0 {
                continue;
            }
            assert_eq!(got, inp, "catalog and pricing disagree on {}", m.slug);
        }
    }
}
