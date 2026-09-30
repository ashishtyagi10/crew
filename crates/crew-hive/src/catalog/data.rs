//! The curated catalog rows. Split from `catalog.rs` to keep both under the
//! line cap. Prices are µ$/Mtok list rates, 2026-09; `None` means we don't
//! have a verified rate (the picker badges it `—` and live enrichment may
//! fill it in). Free rows are OpenRouter `:free` variants.
use super::{ModelInfo, Vendor};

const fn m(
    name: &'static str,
    slug: &'static str,
    or_slug: Option<&'static str>,
    vendor: Vendor,
    price: Option<(u64, u64)>,
    free: bool,
    context: u32,
) -> ModelInfo {
    ModelInfo {
        name,
        slug,
        or_slug,
        vendor,
        price,
        free,
        context,
    }
}

const M: u64 = 1_000_000; // µ$ per $1

// rustfmt::skip keeps each row a single scannable line.
#[rustfmt::skip]
pub(super) const MODELS: &[ModelInfo] = &[
    // Anthropic, newest first — rates verified against Anthropic's model
    // table, 2026-09; OpenRouter aliases read off its public `/models` list
    // (2026-09-29), `None` where it lists no such model.
    m("Claude Opus 5.5", "claude-opus-5-5", Some("anthropic/claude-opus-5.5"), Vendor::Anthropic, Some((4 * M, 20 * M)), false, 1_000_000),
    m("Claude Sonnet 5.5", "claude-sonnet-5-5", Some("anthropic/claude-sonnet-5.5"), Vendor::Anthropic, Some((2 * M, 10 * M)), false, 1_000_000),
    m("Claude Fable 5.1", "claude-fable-5-1", Some("anthropic/claude-fable-5.1"), Vendor::Anthropic, Some((10 * M, 50 * M)), false, 1_000_000),
    m("Claude Opus 5", "claude-opus-5", Some("anthropic/claude-opus-5"), Vendor::Anthropic, Some((5 * M, 25 * M)), false, 1_000_000),
    m("Claude Sonnet 5", "claude-sonnet-5", Some("anthropic/claude-sonnet-5"), Vendor::Anthropic, Some((2 * M, 10 * M)), false, 1_000_000),
    m("Claude Haiku 4.5", "claude-haiku-4-5", Some("anthropic/claude-haiku-4.5"), Vendor::Anthropic, Some((M, 5 * M)), false, 200_000),
    m("Claude Opus 4.8", "claude-opus-4-8", Some("anthropic/claude-opus-4.8"), Vendor::Anthropic, Some((5 * M, 25 * M)), false, 1_000_000),
    m("Claude Fable 5", "claude-fable-5", Some("anthropic/claude-fable-5"), Vendor::Anthropic, Some((10 * M, 50 * M)), false, 1_000_000),
    // OpenAI — rates from `pricing::RATES`. The GPT-6 rows are pickable but
    // are not the `openai` default: their model pages (2026-09-29) allow
    // function tools on Chat Completions only at `reasoning_effort: none`
    // (Sol, Luna) or not at all (6.1 Sol, Astra) — see `DIRECT`.
    m("GPT-6.1 Sol", "gpt-6.1-sol", Some("openai/gpt-6.1-sol"), Vendor::OpenAI, Some((2 * M, 10 * M)), false, 1_050_000),
    m("GPT-6 Sol", "gpt-6-sol", Some("openai/gpt-6-sol"), Vendor::OpenAI, Some((2 * M, 10 * M)), false, 1_050_000),
    m("GPT-6 Luna", "gpt-6-luna", Some("openai/gpt-6-luna"), Vendor::OpenAI, Some((100_000, 500_000)), false, 1_050_000),
    m("GPT-6 Astra", "gpt-6-astra", Some("openai/gpt-6-astra"), Vendor::OpenAI, Some((10 * M, 50 * M)), false, 1_050_000),
    m("GPT-5.5", "gpt-5.5", Some("openai/gpt-5.5"), Vendor::OpenAI, Some((5 * M, 30 * M)), false, 1_050_000),
    m("GPT-5", "gpt-5", Some("openai/gpt-5"), Vendor::OpenAI, Some((1_250_000, 10 * M)), false, 400_000),
    m("GPT-4.1", "gpt-4.1", Some("openai/gpt-4.1"), Vendor::OpenAI, Some((2 * M, 8 * M)), false, 0),
    m("GPT-4.1 Mini", "gpt-4.1-mini", Some("openai/gpt-4.1-mini"), Vendor::OpenAI, Some((400_000, 1_600_000)), false, 0),
    m("GPT-4o", "gpt-4o", Some("openai/gpt-4o"), Vendor::OpenAI, Some((2_500_000, 10 * M)), false, 0),
    m("GPT-4o Mini", "gpt-4o-mini", Some("openai/gpt-4o-mini"), Vendor::OpenAI, Some((150_000, 600_000)), false, 0),
    // Alibaba / DashScope — rates from `pricing::RATES`. OpenRouter lists
    // only the qwen3.8-max SNAPSHOT (`-0902`), not the alias, so no alias.
    m("Qwen3.8 Max", "qwen3.8-max", None, Vendor::Alibaba, Some((2 * M, 6 * M)), false, 1_000_000),
    m("Qwen3.8 Flash", "qwen3.8-flash", Some("qwen/qwen3.8-flash"), Vendor::Alibaba, Some((150_000, 470_000)), false, 1_000_000),
    m("Qwen3.7 Max", "qwen3.7-max", Some("qwen/qwen3.7-max"), Vendor::Alibaba, Some((2_500_000, 7_500_000)), false, 1_000_000),
    m("Qwen Max", "qwen-max", Some("qwen/qwen3-max"), Vendor::Alibaba, Some((1_600_000, 6_400_000)), false, 0),
    m("Qwen Plus", "qwen-plus", Some("qwen/qwen-plus"), Vendor::Alibaba, Some((400_000, 1_200_000)), false, 0),
    m("Qwen Turbo", "qwen-turbo", None, Vendor::Alibaba, Some((50_000, 200_000)), false, 0),
    m("Qwen3 Coder Plus", "qwen3-coder-plus", None, Vendor::Alibaba, Some((M, 5 * M)), false, 0),
    m("Qwen3 Coder Flash", "qwen3-coder-flash", None, Vendor::Alibaba, Some((300_000, 1_500_000)), false, 0),
    // DeepSeek / Moonshot — rates from `pricing::RATES`. `deepseek-chat` and
    // `deepseek-reasoner` were retired 2026-07-24, so they are no longer rows;
    // `deepseek-flash` serves V4.1 Flash, OpenRouter's `deepseek-v4.1-flash`.
    m("DeepSeek Flash", "deepseek-flash", Some("deepseek/deepseek-v4.1-flash"), Vendor::DeepSeek, Some((300_000, 1_200_000)), false, 1_000_000),
    m("DeepSeek V4 Pro", "deepseek-v4-pro", Some("deepseek/deepseek-v4-pro"), Vendor::DeepSeek, Some((1_320_000, 3_960_000)), false, 1_000_000),
    m("Kimi K2", "kimi-k2", Some("moonshotai/kimi-k2"), Vendor::Moonshot, Some((600_000, 2_500_000)), false, 0),
    // Meta — rates unknown; enrichment fills these.
    m("Llama 3.3 70B", "meta-llama/llama-3.3-70b-instruct", Some("meta-llama/llama-3.3-70b-instruct"), Vendor::Meta, None, false, 131_072),
    // Google — rates from `pricing::RATES`; 2.5 Flash's left to enrichment.
    // 2.5 is limited to projects that already used it (2026-09-18).
    m("Gemini 3.8 Flash", "gemini-3.8-flash", Some("google/gemini-3.8-flash"), Vendor::Google, Some((750_000, 3_750_000)), false, 1_048_576),
    m("Gemini 3.1 Pro Preview", "gemini-3.1-pro-preview", Some("google/gemini-3.1-pro-preview"), Vendor::Google, Some((2 * M, 12 * M)), false, 1_048_576),
    m("Gemini 2.5 Pro", "gemini-2.5-pro", Some("google/gemini-2.5-pro"), Vendor::Google, Some((1_250_000, 10 * M)), false, 1_048_576),
    m("Gemini 2.5 Flash", "gemini-2.5-flash", Some("google/gemini-2.5-flash"), Vendor::Google, None, false, 0),
    // NVIDIA NIM — native ids from `integrate.api.nvidia.com/v1/models`
    // (2026-09-08), served by the `nvidia` direct provider; the free
    // developer tier has no per-token rate, so no price is claimed. The
    // OpenRouter aliases are the paid slugs (the `:free` twins are below).
    m("Nemotron 3.5 Lightning", "nvidia/nemotron-3.5-lightning-30b-a3b", Some("nvidia/nemotron-3.5-lightning"), Vendor::Nvidia, None, false, 1_000_000),
    m("Nemotron 3 Super", "nvidia/nemotron-3-super-120b-a12b", Some("nvidia/nemotron-3-super-120b-a12b"), Vendor::Nvidia, None, false, 1_000_000),
    m("Nemotron 3 Ultra 550B", "nvidia/nemotron-3-ultra-550b-a55b", Some("nvidia/nemotron-3-ultra-550b-a55b"), Vendor::Nvidia, None, false, 1_000_000),
    // Free tier — verified live on OpenRouter's public `/models` endpoint
    // (2026-07-25), spanning different vendors so a provider-specific throttle
    // doesn't collapse the entire fallback chain. Unlike the `context: 0`
    // rows above (unconfirmed), these `context` values were read straight
    // off that live response's `context_length` field (the 3.5 Lightning
    // row re-read 2026-09-08), not guessed — keep them in sync with OpenRouter if the rows are refreshed.
    // Re-read 2026-09-29: `openai/gpt-oss-20b:free` was gone from the list,
    // so Qwen3.8 27B holds its place as a fourth vendor.
    m("Nemotron 3.5 Lightning", "nvidia/nemotron-3.5-lightning:free", Some("nvidia/nemotron-3.5-lightning:free"), Vendor::Nvidia, Some((0, 0)), true, 1_000_000),
    m("Nemotron 3 Ultra", "nvidia/nemotron-3-ultra-550b-a55b:free", Some("nvidia/nemotron-3-ultra-550b-a55b:free"), Vendor::Nvidia, Some((0, 0)), true, 1_000_000),
    m("Qwen3.8 27B", "qwen/qwen3.8-27b:free", Some("qwen/qwen3.8-27b:free"), Vendor::Alibaba, Some((0, 0)), true, 262_144),
    m("Gemma 4 31B", "google/gemma-4-31b-it:free", Some("google/gemma-4-31b-it:free"), Vendor::Google, Some((0, 0)), true, 262_144),
    m("North Mini Code", "cohere/north-mini-code:free", Some("cohere/north-mini-code:free"), Vendor::Cohere, Some((0, 0)), true, 256_000),
];
