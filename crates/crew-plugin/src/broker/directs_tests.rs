//! The default chains name models their vendors serve today. Each pin here
//! is a model read off the vendor's own docs (2026-09-29); change one only
//! after re-reading them, since a chain head that 404s — as `gemini-2.5-pro`
//! did for a new key, and `deepseek-chat` did for everyone — makes the whole
//! provider look broken on its first call.
use super::{direct_by_name, DIRECT};

fn chain(name: &str) -> &'static [&'static str] {
    direct_by_name(name).expect("a DIRECT row").chain
}

/// Google limited Gemini 2.5 to projects that already used it, so a GA
/// Gemini 3 model leads; a preview may back it up but never lead; 2.5 Pro
/// stays LAST, for the keys that still reach it.
#[test]
fn gemini_leads_with_a_ga_model_and_keeps_2_5_last() {
    let c = chain("gemini");
    assert_eq!(c[0], "gemini-3.8-flash");
    assert!(!c[0].contains("preview"));
    assert_eq!(c.last(), Some(&"gemini-2.5-pro"));
}

/// OpenAI's head must take function tools on Chat Completions at the default
/// reasoning effort, which rules the GPT-6 family out (see `DIRECT`).
#[test]
fn openai_leads_with_the_newest_model_that_takes_tools_on_chat_completions() {
    assert_eq!(chain("openai"), ["gpt-5.5", "gpt-5", "gpt-4.1"]);
    assert!(DIRECT
        .iter()
        .all(|d| d.chain.iter().all(|m| !m.starts_with("gpt-6"))));
}

/// `deepseek-chat` / `deepseek-reasoner` were retired 2026-07-24.
#[test]
fn deepseek_no_longer_names_its_retired_models() {
    assert_eq!(chain("deepseek"), ["deepseek-flash", "deepseek-v4-pro"]);
}

/// The DashScope chain answers to the same rule as the `DIRECT` rows
/// (`chains_are_native_catalog_slugs`): every slug a native Alibaba row. It
/// still leads with qwen-max: qwen3.8-max thinks first and answers a plain
/// question about twice as late, so it is catalogued, not the default.
#[test]
fn dashscope_chain_is_catalogued_and_leads_with_qwen_max() {
    let c = crate::broker::discover::DEFAULT_DASHSCOPE_CHAIN;
    assert_eq!(c[0], "qwen-max");
    assert!(crew_hive::catalog::catalog()
        .iter()
        .any(|m| m.slug == "qwen3.8-max"));
    for slug in c {
        let known = crew_hive::catalog::catalog()
            .iter()
            .any(|m| m.vendor == crew_hive::catalog::Vendor::Alibaba && m.slug == *slug && !m.free);
        assert!(known, "{slug} is not a native Alibaba catalog slug");
    }
}
