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
/// Gemini 3 model leads; a preview may back it up but never lead.
#[test]
fn gemini_leads_with_a_ga_model() {
    let c = chain("gemini");
    assert_eq!(c[0], "gemini-3.8-flash");
    assert!(!c[0].contains("preview"));
}

/// OpenAI's head must take function tools on Chat Completions at the default
/// reasoning effort, which rules the GPT-6 family out of the lead (see
/// `DIRECT`). The one GPT-6 rung is Luna, last, because crew's body seam
/// sends it the `reasoning_effort: none` its tools need — no other GPT-6
/// model gets that, so no other may appear.
#[test]
fn openai_leads_with_the_newest_model_that_takes_tools_on_chat_completions() {
    assert_eq!(chain("openai"), ["gpt-5.5", "gpt-4.1", "gpt-6-luna"]);
    for d in DIRECT {
        assert!(!d.chain[0].starts_with("gpt-6"), "{}", d.name);
        for m in d.chain.iter().filter(|m| m.starts_with("gpt-6")) {
            assert_eq!(*m, "gpt-6-luna", "{}: no effort seam for {m}", d.name);
        }
    }
}

/// When `gpt-4.1` is shut down (2027-01-14) the chain must still hold a rung
/// past it that OpenAI has not retired: its named replacement, Luna.
#[test]
fn openai_keeps_a_rung_past_the_gpt_4_1_shutdown() {
    let c = chain("openai");
    let after = c.iter().position(|m| *m == "gpt-4.1").map_or(0, |i| i + 1);
    assert!(c[after..].contains(&"gpt-6-luna"), "{c:?}");
}

/// No chain names a model its vendor has retired or set a shutdown date for
/// within a month (each vendor's deprecations page, read 2026-11-22): a rung
/// that 404s costs a failed call before the next one answers.
#[test]
fn no_chain_names_a_retiring_model() {
    const RETIRING: &[&str] = &[
        "gpt-5",          // OpenAI shuts it down 2026-12-11
        "gemini-2.5-pro", // routed to 3.8 Flash from 2026-12-01
        "deepseek-chat",  // retired 2026-07-24
        "deepseek-reasoner",
        "qwen-turbo", // decommissioned 2026-10-10
    ];
    let dashscope = (
        "dashscope",
        crate::broker::discover::DEFAULT_DASHSCOPE_CHAIN,
    );
    for (name, c) in DIRECT.iter().map(|d| (d.name, d.chain)).chain([dashscope]) {
        for m in c {
            assert!(!RETIRING.contains(m), "{name}: {m}");
        }
    }
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
