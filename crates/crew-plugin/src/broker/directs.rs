//! The `DIRECT` provider table: every vendor crew reaches over the OpenAI
//! chat-completions wire with nothing but an endpoint, a key variable and a
//! default model chain. Split from `discover.rs` to keep that file inside
//! the line cap as the OAuth-grant plumbing grew in; `discover` re-exports
//! everything, so callers never learned the move happened.

/// A provider crew reaches over the OpenAI chat-completions wire, which is
/// most of them. Adding one is a row here: an endpoint, a key variable and a
/// default model chain. `OpenRouterProvider` already speaks this wire — it
/// has backed DashScope from the beginning with nothing but a different base
/// URL — so no client code is involved.
///
/// This exists because OpenRouter was the only multi-vendor route crew had,
/// which made `OPENROUTER_API_KEY` the answer for every OpenAI, Google or
/// Mistral row in the picker whether or not the user held a key for the
/// vendor itself.
pub struct DirectProvider {
    /// As `CREW_PROVIDER` and the credential store spell it.
    pub name: &'static str,
    pub var: &'static str,
    pub endpoint: &'static str,
    pub chain: &'static [&'static str],
    /// Comma-separated override, e.g. `CREW_OPENAI_MODEL=gpt-5,gpt-4.1`.
    pub chain_env: &'static str,
    pub base_url_env: &'static str,
    /// The catalog vendor this provider serves natively, so the model picker
    /// can route a row to it instead of to OpenRouter.
    pub vendor: crew_hive::catalog::Vendor,
}

/// Every OpenAI-wire provider crew knows, in discovery order.
///
/// Model ids are NOT invented here. Every slug in every chain is a native
/// (non-OpenRouter) slug that already exists in `crew_hive::catalog` for that
/// vendor, and `chains_are_native_catalog_slugs` enforces it — a default model
/// that 404s on first use is a worse first run than no provider at all. That
/// is also why xAI, Mistral and Groq are absent despite speaking this same
/// wire: the catalog carries no rows for them, so their ids would be guesses.
pub static DIRECT: &[DirectProvider] = &[
    DirectProvider {
        name: "openai",
        var: "OPENAI_API_KEY",
        endpoint: "https://api.openai.com/v1/chat/completions",
        chain: &["gpt-5", "gpt-4.1"],
        chain_env: "CREW_OPENAI_MODEL",
        base_url_env: "CREW_OPENAI_BASE_URL",
        vendor: crew_hive::catalog::Vendor::OpenAI,
    },
    DirectProvider {
        name: "gemini",
        var: "GEMINI_API_KEY",
        // Google's own OpenAI-compatibility endpoint, so the same client
        // works — no Google SDK, no second wire format.
        endpoint: "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions",
        chain: &["gemini-2.5-pro", "gemini-2.5-flash"],
        chain_env: "CREW_GEMINI_MODEL",
        base_url_env: "CREW_GEMINI_BASE_URL",
        vendor: crew_hive::catalog::Vendor::Google,
    },
    DirectProvider {
        name: "deepseek",
        var: "DEEPSEEK_API_KEY",
        endpoint: "https://api.deepseek.com/chat/completions",
        chain: &["deepseek-chat", "deepseek-reasoner"],
        chain_env: "CREW_DEEPSEEK_MODEL",
        base_url_env: "CREW_DEEPSEEK_BASE_URL",
        vendor: crew_hive::catalog::Vendor::DeepSeek,
    },
];

/// The `DIRECT` row named by `name`, if any.
pub fn direct_by_name(name: &str) -> Option<&'static DirectProvider> {
    DIRECT.iter().find(|d| d.name.eq_ignore_ascii_case(name))
}

impl Clone for DirectProvider {
    fn clone(&self) -> Self {
        unreachable!("DirectProvider lives in a static table and is never cloned")
    }
}

impl PartialEq for DirectProvider {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl Eq for DirectProvider {}
impl std::fmt::Debug for DirectProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Direct({})", self.name)
    }
}
