//! Models the vendors have dated for shutdown. WHY: `gpt-5`, `gpt-4o` and
//! `gemini-2.5-pro` stayed pickable rows with nothing on them, and a model
//! pinned through `CREW_OPENAI_MODEL` would simply start failing on the day,
//! read as a crew bug. The picker badges these rows and the broker says once,
//! under a reply, when the model that served it is near its date.
//!
//! Only dates read off a vendor page go in [`RULES`]; a guess here would
//! badge a healthy model. Re-read the pages when this is refreshed.

/// What a vendor said about one model id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Notice {
    /// The shutdown (or re-route) day, (year, month, day).
    pub date: (i32, u32, u32),
    /// The replacement the vendor recommends.
    pub next: &'static str,
    /// The vendor routes calls to `next` from `date` rather than failing
    /// them: the model is "replaced", not "retired".
    pub routed: bool,
    /// Days from today to `date`; 0 on the day, negative after it.
    pub days_left: i64,
}

/// How near the date a served model starts being mentioned under replies.
pub const WARN_DAYS: i64 = 14;

struct Rule(&'static str, (i32, u32, u32), &'static str, bool);

#[rustfmt::skip]
const RULES: &[Rule] = &[
    // OpenAI — developers.openai.com/api/docs/deprecations, read 2026-11-24.
    Rule("gpt-5", (2026, 12, 11), "gpt-6.1-sol", false),
    Rule("gpt-5-mini", (2026, 12, 11), "gpt-6.1-terra", false),
    Rule("gpt-5-nano", (2026, 12, 11), "gpt-6-luna", false),
    Rule("gpt-5-pro", (2026, 12, 11), "gpt-6.1-sol", false),
    Rule("o3", (2026, 12, 11), "gpt-6.1-sol", false),
    Rule("o3-pro", (2026, 12, 11), "gpt-6.1-sol", false),
    Rule("gpt-4.1", (2027, 1, 14), "gpt-6-luna", false),
    Rule("gpt-4o", (2027, 1, 14), "gpt-6-luna", false),
    Rule("gpt-4o-mini", (2027, 1, 14), "gpt-6-luna", false),
    Rule("gpt-4.1-mini", (2027, 1, 14), "gpt-6-luna", false),
    // Anthropic — platform.claude.com/docs/en/about-claude/model-deprecations,
    // read 2026-11-24 (also .claude/commands/crew.md "What is current").
    Rule("claude-sonnet-4-5", (2026, 11, 30), "claude-sonnet-5-5", false),
    Rule("claude-opus-4-5", (2027, 1, 22), "claude-opus-5-5", false),
    // Google — aistudio.google.com/docs/deprecations, read 2026-11-24: API
    // calls route to the replacement from the date.
    Rule("gemini-2.5-pro", (2026, 12, 1), "gemini-3.8-flash", true),
    Rule("gemini-2.5-flash", (2026, 12, 1), "gemini-3.5-flash-lite", true),
];

/// The vendor's notice for `id` as of `today` (days since 1970-01-01, UTC),
/// or `None` when no vendor has dated it.
///
/// `id` matches a rule exactly, or as that rule plus a dated snapshot suffix
/// (`gpt-5-2025-08-07`, `claude-sonnet-4-5-20250929`) — never as a prefix,
/// or `gpt-5` would badge `gpt-5.5` and `gpt-4o` would badge `gpt-4o-mini`.
/// An OpenRouter alias (`openai/gpt-5`) does not match: a reseller sets its
/// own dates (Anthropic's page says so of partner platforms).
pub fn sunset(id: &str, today: i64) -> Option<Notice> {
    let r = RULES.iter().find(|r| {
        id == r.0
            || id
                .strip_prefix(r.0)
                .and_then(|s| s.strip_prefix('-'))
                .is_some_and(is_date)
    })?;
    let (y, m, d) = r.1;
    Some(Notice {
        date: r.1,
        next: r.2,
        routed: r.3,
        days_left: days_from_civil(y, m, d) - today,
    })
}

/// `2025-08-07` or `20250929`: digits and dashes, eight digits at least.
fn is_date(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_digit() || c == '-')
        && s.chars().filter(char::is_ascii_digit).count() >= 8
}

impl Notice {
    /// Near enough (or past) to say under a reply.
    pub fn soon(&self) -> bool {
        self.days_left <= WARN_DAYS
    }

    /// `Dec 11`.
    pub fn day(&self) -> String {
        const MON: [&str; 12] = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        let (_, m, d) = self.date;
        format!("{} {d}", MON[(m as usize).clamp(1, 12) - 1])
    }

    /// The picker's badge: `retires Dec 11`, or `retired` once the day has
    /// come (the vendor's date is the day calls stop).
    pub fn badge(&self) -> String {
        match (self.days_left > 0, self.routed) {
            (true, false) => format!("retires {}", self.day()),
            (true, true) => format!("replaced {}", self.day()),
            (false, false) => "retired".to_string(),
            (false, true) => "replaced".to_string(),
        }
    }

    /// Smith's line: `gpt-5 retires Dec 11 — gpt-6.1-sol next`.
    pub fn line(&self, id: &str) -> String {
        let when = match (self.days_left > 0, self.routed) {
            (true, false) => "retires",
            (false, false) => "retired",
            (true, true) => "is replaced",
            (false, true) => "was replaced",
        };
        format!("{id} {when} {} \u{2014} {} next", self.day(), self.next)
    }
}

/// Today, as days since 1970-01-01 (UTC) — the one wall-clock read both the
/// picker and the broker use.
pub fn today() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| (d.as_secs() / 86_400) as i64)
}

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant's
/// `days_from_civil`), so a table date and [`today`] compare as integers.
pub fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = i64::from(y) - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = i64::from(m);
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
#[path = "sunset_tests.rs"]
mod tests;
