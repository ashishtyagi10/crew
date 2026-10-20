//! An opened thread, read as text: the subject, then each message's sender
//! and time over its words, oldest first, attachments named under it.
use super::draw::{wrap, Canvas};
use super::wire::sender_name;
use super::Reading;

/// One laid-out line: text, whether it is secondary, whether it is bold.
pub(crate) type Line = (String, bool, bool);

pub(crate) fn layout(r: &Reading, width: usize) -> Vec<Line> {
    let subject = match r.thread.subject.trim() {
        "" => "(no subject)",
        s => s,
    };
    let mut out: Vec<Line> = wrap(subject, width)
        .into_iter()
        .map(|l| (l, false, true))
        .collect();
    let Some(mails) = &r.mails else {
        out.push((String::new(), false, false));
        out.push(("loading\u{2026}".into(), true, false));
        return out;
    };
    for m in mails {
        out.push((String::new(), false, false));
        out.push((
            format!("{} \u{b7} {}", sender_name(&m.from), when(&m.date)),
            true,
            true,
        ));
        out.extend(wrap(&m.text, width).into_iter().map(|l| (l, false, false)));
        if !m.files.is_empty() {
            out.push((format!("attached: {}", m.files.join(", ")), true, false));
        }
    }
    out
}

pub(crate) fn draw(cv: &mut Canvas, r: &Reading) {
    let t = crew_theme::theme();
    let width = cv.cols.saturating_sub(4) as usize;
    let room = cv.rows.saturating_sub(1) as usize;
    for (i, (text, quiet, bold)) in layout(r, width)
        .iter()
        .skip(r.scroll)
        .take(room)
        .enumerate()
    {
        let fg = if *quiet { t.text_muted } else { t.ink };
        cv.text(i as u16, 2, cv.cols, text, fg, *bold);
    }
}

/// A `Date` header in local time, `Tue Oct 20, 09:14`; as written when it
/// does not parse. Mailers add a zone comment (`+0000 (UTC)`) or a weekday
/// that disagrees with the date, and RFC 2822 parsing refuses both, so each
/// is tried without them too.
pub(crate) fn when(date: &str) -> String {
    let d = date.trim();
    let bare = d.split(" (").next().unwrap_or(d);
    let dayless = bare.split_once(", ").map_or(bare, |(_, rest)| rest);
    [d, bare, dayless]
        .iter()
        .find_map(|s| chrono::DateTime::parse_from_rfc2822(s).ok())
        .map(|t| {
            t.with_timezone(&chrono::Local)
                .format("%a %b %-d, %H:%M")
                .to_string()
        })
        .unwrap_or_else(|| d.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_date_header_reads_in_local_time_whatever_the_mailer_added() {
        for d in [
            "Tue, 20 Oct 2026 13:02:00 +0000",
            "Tue, 20 Oct 2026 13:02:00 +0000 (UTC)",
            "Mon, 20 Oct 2026 13:02:00 +0000",
        ] {
            let got = super::when(d);
            assert!(
                got.contains("Oct 20") || got.contains("Oct 21"),
                "{d} -> {got}"
            );
        }
        assert_eq!(super::when("  yesterday "), "yesterday");
    }
}
