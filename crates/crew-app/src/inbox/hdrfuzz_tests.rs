//! Mail headers from anyone, mangled at random: the parsers never panic.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

const BITS: &[&str] = &[
    "<",
    ">",
    "\"",
    ",",
    ";",
    "@",
    " ",
    "\t",
    "\r\n",
    "mailto:",
    "https://",
    "?subject=",
    "&body=",
    "%",
    "%ZZ",
    "%F0%9F",
    "=?UTF-8?B?",
    "?=",
    "\u{1F600}",
    "é",
    "中",
    "\u{202e}",
    "(",
    ")",
    "Ann",
    "a@b.co",
    ".",
    "Mon, ",
    "31 Feb 2026",
    " +9999",
    " (UTC)",
    "One-Click",
    "List-Unsubscribe=",
    "\\",
    "'",
    "]",
    "[",
    "\u{0}",
    "\u{feff}",
];

/// Addresses, List-Unsubscribe, dates and subjects made of the pieces that
/// break parsers: none panics, and the reply box's line editing neither.
#[test]
fn any_mangled_header_parses_without_a_panic() {
    let mut fails = Vec::new();
    for seed in 1..4000u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
        let n = rng.next() % 12;
        let s: String = (0..n)
            .map(|_| BITS[(rng.next() % BITS.len() as u64) as usize])
            .collect();
        let t = s.clone();
        let r = std::panic::catch_unwind(move || {
            let _ = crate::inbox::addr::split(&t);
            let _ = crate::inbox::addr::first_bad(&t);
            let _ = crate::inbox::addr::header(&t);
            let _ = crate::inbox::addr::names(&t);
            let _ = crate::inbox::addr::valid(&t);
            let _ = crate::inbox::wire::sender_name(&t);
            let _ = crate::inbox::reader::when(&t);
            let _ = crate::inbox::forward::subject(&t);
            let msg = serde_json::json!({"payload": {"headers": [
                {"name": "List-Unsubscribe", "value": t},
                {"name": "List-Unsubscribe-Post", "value": "List-Unsubscribe=One-Click"},
                {"name": "Authentication-Results", "value": "dkim=pass"},
                {"name": "From", "value": t}, {"name": "Subject", "value": t}]}});
            let _ = crate::inbox::unsub::of(&msg);
            let _ = crate::inbox::draft::Reply::of(&msg);
            let mut f = crate::inbox::field::Field::with(&t);
            for op in 0..12 {
                match op % 6 {
                    0 => f.left(),
                    1 => f.home(),
                    2 => f.back(),
                    3 => {
                        f.line(true);
                    }
                    4 => f.end(),
                    _ => f.put(&t, op % 2 == 0),
                }
            }
        });
        if r.is_err() {
            fails.push(s);
        }
    }
    assert!(
        fails.is_empty(),
        "{} panics: {:?}",
        fails.len(),
        &fails[..fails.len().min(5)]
    );
}
