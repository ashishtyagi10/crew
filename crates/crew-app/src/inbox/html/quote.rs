//! Where a reply's quoted history starts, as each mailer marks it.
//!
//! In a thread the earlier messages are already on screen above, so the
//! history a reply carries is a copy; the reader folds it to one row (goal
//! 2026-10-20-mail-reader, item 6). Each mailer has its own marker:
//! Gmail's `gmail_quote`, Apple Mail's `blockquote type=cite`, Thunderbird's
//! `moz-cite-prefix`, Yahoo's `yahoo_quoted`, and Outlook's
//! `divRplyFwdMsg` or the bordered `From:` block its desktop app draws.
use super::tree::Node;

pub(crate) fn starts(n: &Node) -> bool {
    let class = n.attr("class").unwrap_or("").to_ascii_lowercase();
    let id = n.attr("id").unwrap_or("").to_ascii_lowercase();
    if class.split_whitespace().any(|c| {
        c == "gmail_quote"
            || c == "gmail_quote_container"
            || c == "moz-cite-prefix"
            || c == "yahoo_quoted"
    }) {
        return true;
    }
    if id.contains("divrplyfwdmsg") || id == "appendonsend" {
        return true;
    }
    if n.tag == "blockquote"
        && n.attr("type")
            .is_some_and(|t| t.eq_ignore_ascii_case("cite"))
    {
        return true;
    }
    n.tag == "div"
        && n.attr("style")
            .is_some_and(|s| s.to_ascii_lowercase().contains("border-top"))
        && n.text().trim_start().starts_with("From:")
}

/// Whether the history is a forwarded mail: then it is the content, not a
/// copy, and stays open.
pub(crate) fn forwarded(quote: &str) -> bool {
    let head: String = quote.chars().take(400).collect::<String>().to_lowercase();
    head.contains("forwarded message") || head.contains("begin forwarded message")
}
