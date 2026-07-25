//! The curated catalog rows. Split from `catalog.rs` to keep both under the
//! line cap. Prices are µ$/Mtok list rates, 2026-07; `None` means we don't
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
    // Anthropic — rates verified against the 2026-07 first-party card.
    m("Claude Opus 5", "claude-opus-5", Some("anthropic/claude-opus-5"), Vendor::Anthropic, Some((5 * M, 25 * M)), false, 1_000_000),
    m("Claude Sonnet 5", "claude-sonnet-5", Some("anthropic/claude-sonnet-5"), Vendor::Anthropic, Some((3 * M, 15 * M)), false, 1_000_000),
    m("Claude Haiku 4.5", "claude-haiku-4-5", Some("anthropic/claude-haiku-4.5"), Vendor::Anthropic, Some((M, 5 * M)), false, 200_000),
    m("Claude Opus 4.8", "claude-opus-4-8", Some("anthropic/claude-opus-4.8"), Vendor::Anthropic, Some((5 * M, 25 * M)), false, 1_000_000),
    m("Claude Fable 5", "claude-fable-5", None, Vendor::Anthropic, Some((10 * M, 50 * M)), false, 1_000_000),
    // OpenAI — rates from `pricing::RATES`; GPT-5 list rate unverified.
    m("GPT-4.1", "gpt-4.1", Some("openai/gpt-4.1"), Vendor::OpenAI, Some((2 * M, 8 * M)), false, 0),
    m("GPT-4.1 Mini", "gpt-4.1-mini", Some("openai/gpt-4.1-mini"), Vendor::OpenAI, Some((400_000, 1_600_000)), false, 0),
    m("GPT-4o", "gpt-4o", Some("openai/gpt-4o"), Vendor::OpenAI, Some((2_500_000, 10 * M)), false, 0),
    m("GPT-4o Mini", "gpt-4o-mini", Some("openai/gpt-4o-mini"), Vendor::OpenAI, Some((150_000, 600_000)), false, 0),
    m("GPT-5", "gpt-5", Some("openai/gpt-5"), Vendor::OpenAI, None, false, 0),
    // Alibaba / DashScope — rates from `pricing::RATES`.
    m("Qwen Max", "qwen-max", Some("qwen/qwen3-max"), Vendor::Alibaba, Some((1_600_000, 6_400_000)), false, 0),
    m("Qwen Plus", "qwen-plus", Some("qwen/qwen-plus"), Vendor::Alibaba, Some((400_000, 1_200_000)), false, 0),
    m("Qwen Turbo", "qwen-turbo", None, Vendor::Alibaba, Some((50_000, 200_000)), false, 0),
    m("Qwen3 Coder Plus", "qwen3-coder-plus", None, Vendor::Alibaba, Some((M, 5 * M)), false, 0),
    m("Qwen3 Coder Flash", "qwen3-coder-flash", None, Vendor::Alibaba, Some((300_000, 1_500_000)), false, 0),
    // DeepSeek / Moonshot — rates from `pricing::RATES`.
    m("DeepSeek Chat", "deepseek-chat", Some("deepseek/deepseek-chat"), Vendor::DeepSeek, Some((270_000, 1_100_000)), false, 0),
    m("DeepSeek Reasoner", "deepseek-reasoner", Some("deepseek/deepseek-r1"), Vendor::DeepSeek, Some((550_000, 2_190_000)), false, 0),
    m("Kimi K2", "kimi-k2", Some("moonshotai/kimi-k2"), Vendor::Moonshot, Some((600_000, 2_500_000)), false, 0),
    // Meta — rates unknown; enrichment fills these.
    m("Llama 3.3 70B", "meta-llama/llama-3.3-70b-instruct", Some("meta-llama/llama-3.3-70b-instruct"), Vendor::Meta, None, false, 131_072),
    // Google — no verified first-party rate in-repo; enrichment fills these.
    m("Gemini 2.5 Pro", "gemini-2.5-pro", Some("google/gemini-2.5-pro"), Vendor::Google, None, false, 0),
    m("Gemini 2.5 Flash", "gemini-2.5-flash", Some("google/gemini-2.5-flash"), Vendor::Google, None, false, 0),
    // Free tier — verified live on OpenRouter's public `/models` endpoint
    // (2026-07-25), spanning different vendors so a provider-specific throttle
    // doesn't collapse the entire fallback chain.
    m("Nemotron 3 Ultra", "nvidia/nemotron-3-ultra-550b-a55b:free", Some("nvidia/nemotron-3-ultra-550b-a55b:free"), Vendor::Nvidia, Some((0, 0)), true, 1_000_000),
    m("GPT-OSS 20B", "openai/gpt-oss-20b:free", Some("openai/gpt-oss-20b:free"), Vendor::OpenAI, Some((0, 0)), true, 131_072),
    m("Gemma 4 31B", "google/gemma-4-31b-it:free", Some("google/gemma-4-31b-it:free"), Vendor::Google, Some((0, 0)), true, 262_144),
    m("North Mini Code", "cohere/north-mini-code:free", Some("cohere/north-mini-code:free"), Vendor::Cohere, Some((0, 0)), true, 256_000),
];
