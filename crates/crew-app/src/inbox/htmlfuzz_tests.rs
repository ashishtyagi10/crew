//! Mail from anyone, mangled at random: the reader never panics on it (a
//! panic would take the app down as the mail opened).
use crate::inbox::api::Mail;
use crate::inbox::rich::Rich;
use crate::inbox::wire::Thread;
use crate::inbox::Reading;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

const SNIPS: &[&str] = &[
    "<table><tr><td colspan=999999 rowspan=-3>x</td></tr></table>",
    "<a href='",
    "&#xFFFFFFFF;",
    "&#0;",
    "&#55296;",
    "<style>",
    "<!--",
    "<![CDATA[",
    "</td></tr></table></div>",
    "<td>",
    "<tr>",
    "<img src=cid: width=99999999999 height=-5>",
    "<img src='data:image/png;base64,@@@'>",
    "<div style='font-size:-1px;font-weight:9999;text-transform:uppercase'>",
    "<span style='display:none'>",
    "\u{202e}",
    "\u{200d}\u{200d}",
    "<h1></h1>",
    "<blockquote>",
    "<pre>\t\t</pre>",
    "<li><li><li>",
    "<ol start=-9>",
    "<br/><br/><br/>",
    "<p style='padding:999px'>",
    "<a href='javascript:x'>b</a>",
    "\u{0915}\u{094d}\u{0937}",
    "<table border=0 cellpadding=0><tr><td><table><tr><td>",
    "\r\n\r\n",
    "<<<>>>",
    "&amp;&amp;;",
];

/// Real mails from the corpus, cut, spliced and laced with hostile HTML
/// (huge spans, bad entities, unclosed comments, absurd CSS), read and laid
/// out at every width: none panics. Ten thousand of these ran clean once.
#[test]
fn any_mangled_mail_reads_without_a_panic() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/inbox/html/corpus");
    let mut seeds: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "html"))
        .map(|e| std::fs::read_to_string(e.path()).unwrap())
        .collect();
    seeds.push(String::new());
    let mut fails = Vec::new();
    for seed in 1..250u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
        let mut html = seeds[(rng.next() % seeds.len() as u64) as usize].clone();
        for _ in 0..(rng.next() % 8) {
            let chars: Vec<char> = html.chars().collect();
            let n = chars.len().max(1);
            let at = (rng.next() as usize) % n;
            html = match rng.next() % 4 {
                0 => chars[..at].iter().collect(),
                1 => {
                    let to = (at + (rng.next() as usize % 200)).min(chars.len());
                    chars[..at].iter().chain(chars[to..].iter()).collect()
                }
                _ => {
                    let s = SNIPS[(rng.next() % SNIPS.len() as u64) as usize];
                    chars[..at.min(chars.len())].iter().collect::<String>()
                        + s
                        + &chars[at.min(chars.len())..].iter().collect::<String>()
                }
            };
        }
        let h = html.clone();
        let r = std::panic::catch_unwind(move || {
            let rich = Rich::of_html(&h, None);
            let _ = rich.whole();
            let mail = Mail {
                rich,
                from: "A <a@b.co>".into(),
                ..Mail::default()
            };
            let mut reading = Reading {
                mails: Some(vec![mail]),
                ..Reading::new(Thread::default())
            };
            for (plain, quotes, web) in [(false, false, false), (true, true, true)] {
                reading.plain = plain;
                reading.quotes = quotes;
                reading.web = web;
                for w in [1usize, 3, 8, 20, 47, 88] {
                    *reading.laid.lock().unwrap() = None;
                    let _ = reading.laid(w);
                }
            }
        });
        if r.is_err() {
            fails.push(seed);
        }
    }
    assert!(
        fails.is_empty(),
        "{} panics, seeds {:?}",
        fails.len(),
        &fails[..fails.len().min(20)]
    );
}
