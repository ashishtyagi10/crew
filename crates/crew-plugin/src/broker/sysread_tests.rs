use super::*;
use crate::broker::toolclip::{clip_result, AGENT_CLIP};

fn temp(name: &str, content: &[u8]) -> String {
    let p = std::env::temp_dir().join(format!("crew-sysread-{name}-{}", std::process::id()));
    std::fs::write(&p, content).unwrap();
    p.display().to_string()
}

/// The offset a page's note says to continue from, or None on a last page.
fn next_offset(result: &str) -> Option<usize> {
    let last = result.rsplit('\n').next()?;
    if !last.starts_with("\u{2026} (") || !last.contains("continue with") {
        return None;
    }
    let json = &last[last.rfind('{')?..last.len() - 1];
    let v: serde_json::Value = serde_json::from_str(json).expect("note JSON parses");
    v["offset"].as_u64().map(|n| n as usize)
}

#[test]
fn the_widest_note_still_fits_beside_a_full_page() {
    // Line numbers stop at LINES_UP_TO; offsets can be any u64. Both at their
    // widest, plus the newline before the note, must pass the relay's clip
    // whole — and crew-hive's RESULT_CAP, the same 6,000, keeps the head only.
    let big = LINES_UP_TO + 1;
    let widest = [
        note(
            Some((big - 1, big, big, false)),
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ),
        note(
            Some((big, big, big, true)),
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ),
    ]
    .iter()
    .map(|n| n.chars().count())
    .max()
    .unwrap();
    assert!(
        PAGE + 1 + widest <= AGENT_CLIP,
        "page {PAGE} + note {widest} overflows the {AGENT_CLIP}-char clip"
    );
}

#[test]
fn paging_a_file_through_the_relay_clip_loses_nothing() {
    // ~40 KB: short multibyte lines, one 9,000-byte line of 3-byte chars (so
    // the in-line cut must find a character boundary), and no final newline.
    let mut content = String::new();
    for i in 0..400 {
        content.push_str(&format!(
            "{i:04} fn h\u{e9}llo() {{ /* \u{2713} {} */ }}\n",
            "x".repeat(i % 50)
        ));
    }
    content.push_str(&"\u{65e5}\u{672c}\u{8a9e}".repeat(1_000));
    content.push('\n');
    for i in 0..400 {
        content.push_str(&format!("tail {i}: {}\n", "y".repeat(i % 40)));
    }
    content.push_str("the end, unterminated");
    let path = temp("pages", content.as_bytes());
    let lines = format!(" of {}, ", grouped(content.lines().count()));

    let (mut got, mut offset, mut pages) = (String::new(), 0, 0);
    loop {
        let r = read_file(&path, offset).unwrap();
        assert_eq!(
            clip_result(&r, AGENT_CLIP),
            r,
            "the page at {offset} ({} chars) was clipped before the agent saw it",
            r.chars().count()
        );
        pages += 1;
        let Some(next) = next_offset(&r) else {
            got.push_str(&r);
            break;
        };
        let (page, note) = r.rsplit_once('\n').unwrap();
        assert!(
            note.contains(&lines),
            "note counts the file's lines: {note}"
        );
        assert!(next > offset, "{note} does not move forward from {offset}");
        got.push_str(page);
        offset = next;
    }
    assert!(pages > 5, "a 40 KB file took only {pages} page(s)");
    assert!(got == content, "pages rejoined differ from the file");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_page_ends_on_a_whole_line_and_says_which_lines() {
    let content: String = (1..=1000)
        .map(|i| format!("line {i:04}: some padding text here\n"))
        .collect();
    let path = temp("lines", content.as_bytes());

    let first = read_file(&path, 0).unwrap();
    let (page, note) = first.rsplit_once('\n').unwrap();
    assert!(
        page.ends_with('\n'),
        "page cut mid-line: \u{2026}{}",
        &page[page.len() - 40..]
    );
    let n = page.lines().count();
    assert!(
        note.starts_with(&format!(
            "\u{2026} (lines 1\u{2013}{n} of 1,000, bytes 0\u{2013}"
        )),
        "{note}"
    );
    assert_eq!(next_offset(&first), Some(page.len()), "{note}");

    let second = read_file(&path, page.len()).unwrap();
    assert!(
        second.starts_with(&format!("line {:04}:", n + 1)),
        "{}",
        &second[..20]
    );
    let note = second.rsplit('\n').next().unwrap();
    assert!(
        note.contains(&format!("(lines {}\u{2013}", n + 1)),
        "{note}"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_file_past_the_line_counting_limit_gets_bytes_only() {
    let content = b"0123456789abcde\n".repeat(LINES_UP_TO / 16 + 1);
    let path = temp("huge", &content);
    let r = read_file(&path, 0).unwrap();
    let note = r.rsplit('\n').next().unwrap();
    assert!(note.starts_with("\u{2026} (bytes 0\u{2013}"), "{note}");
    assert!(!note.contains("line"), "{note}");
    assert!(next_offset(&r).is_some(), "{note}");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn grouped_puts_commas_every_three_digits() {
    let got: Vec<String> = [0, 999, 1_000, 40_112, 1_234_567].map(grouped).to_vec();
    assert_eq!(got, ["0", "999", "1,000", "40,112", "1,234,567"]);
}
