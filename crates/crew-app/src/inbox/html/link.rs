//! A mail's links: which ones crew will follow, and the ones that lie.
//!
//! Only `http`, `https` and `mailto` become links; anything else (a
//! `javascript:`, a `data:`, a `file:`, a page-relative `#top`) stays its
//! words. A link whose words name a site and whose address goes to another
//! has the real site drawn after it — `paypal.com ⟶ paypa1-login.xyz` —
//! the check a careful reader does by hovering (goal 2026-10-20-mail-reader,
//! item 7).

/// The address to link to, ready for `[..](<..>)`, when crew follows it.
pub(crate) fn target(href: &str) -> Option<String> {
    let href = super::ent::decode(href.trim());
    let lower = href.to_ascii_lowercase();
    let url = match () {
        _ if lower.starts_with("//") => format!("https:{href}"),
        _ if ["http://", "https://", "mailto:"]
            .iter()
            .any(|s| lower.starts_with(s)) =>
        {
            href
        }
        _ => return None,
    };
    let clean: String = url
        .chars()
        .filter(|c| !c.is_control())
        .map(|c| match c {
            ' ' => "%20".to_string(),
            '<' => "%3C".to_string(),
            '>' => "%3E".to_string(),
            c => c.to_string(),
        })
        .collect();
    (clean.len() > 8).then_some(clean)
}

/// The site a URL goes to, `www.` dropped.
pub(crate) fn host(url: &str) -> Option<String> {
    let lower = url.trim().to_ascii_lowercase();
    let rest = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
        .unwrap_or(&lower);
    let host: String = rest
        .chars()
        .take_while(|c| !matches!(c, '/' | '?' | '#' | ':' | ' '))
        .collect();
    let host = host
        .rsplit('@')
        .next()
        .unwrap_or("")
        .trim_start_matches("www.");
    host.contains('.').then(|| host.to_string())
}

/// When `words` read as a site and `href` goes somewhere else: that place.
pub(crate) fn lie(words: &str, href: &str) -> Option<String> {
    let w = words.trim().to_ascii_lowercase();
    if w.is_empty() || w.contains(char::is_whitespace) || w.contains('@') {
        return None;
    }
    let looks_like_site = w.starts_with("http") || w.starts_with("www.") || {
        let h = host(&w).unwrap_or_default();
        let tld = h.rsplit('.').next().unwrap_or("");
        tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphabetic())
    };
    if !looks_like_site || href.to_ascii_lowercase().starts_with("mailto:") {
        return None;
    }
    let (said, real) = (host(&w)?, host(href)?);
    let same = real == said || real.ends_with(&format!(".{said}"));
    (!same).then_some(real)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_web_and_mail_links_are_followed() {
        assert_eq!(
            target(" https://a.io/x y ").as_deref(),
            Some("https://a.io/x%20y")
        );
        assert_eq!(target("//cdn.io/p").as_deref(), Some("https://cdn.io/p"));
        assert_eq!(
            target("mailto:ann@x.org").as_deref(),
            Some("mailto:ann@x.org")
        );
        assert_eq!(
            target("https://a.io/?a=1&amp;b=2").as_deref(),
            Some("https://a.io/?a=1&b=2")
        );
        for no in [
            "javascript:alert(1)",
            "data:text/html,x",
            "#top",
            "file:///etc",
            "/rel",
        ] {
            assert_eq!(target(no), None, "{no}");
        }
    }

    #[test]
    fn a_link_that_names_another_site_is_caught() {
        assert_eq!(
            lie("paypal.com", "https://paypa1-login.xyz/a").as_deref(),
            Some("paypa1-login.xyz")
        );
        assert_eq!(
            lie("https://www.bank.com/login", "http://evil.io").as_deref(),
            Some("evil.io")
        );
        assert_eq!(lie("amazon.com", "https://www.amazon.com/o/1"), None);
        assert_eq!(lie("amazon.com", "https://click.e.amazon.com/t"), None);
        assert_eq!(lie("Track your order", "https://evil.io"), None);
        assert_eq!(lie("v2.1", "https://x.io"), None, "a version is not a site");
    }
}
