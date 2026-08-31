//! The model catalog behind the `/model` picker: display names, slugs, vendor
//! grouping, list prices, and free/paid marking. Curated here rather than
//! discovered so the picker works offline; `fetch_openrouter` enriches it at
//! runtime where a live rate is available. Prices are µ$ per 1M tokens, the
//! same unit as [`crate::pricing`] — an unknown price is `None`, never a
//! guess (the badge renders `—`).
mod data;
mod live;

pub use live::{fetch as fetch_openrouter, parse_models};

/// One row from the live OpenRouter catalog (owned, unlike [`ModelInfo`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveModel {
    pub id: String,
    pub name: String,
    pub price: Option<(u64, u64)>,
    pub free: bool,
    pub context: u32,
}

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
#[path = "catalog_tests.rs"]
mod tests;
