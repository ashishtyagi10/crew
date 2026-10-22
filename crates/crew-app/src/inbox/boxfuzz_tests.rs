//! The pane under random use: keys, clicks, wheels, pastes and Gmail's
//! answers interleaved, at every size.
use winit::keyboard::{Key, NamedKey};

use crate::inbox::composekeys::tests::reading;

/// A tiny deterministic generator.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

/// Random keys, clicks, wheels, pastes and Gmail's answers, at pane sizes
/// from one cell to a wide screen: nothing panics (a one-row forward box
/// once did, `composedraw`), and nothing it does opens a browser.
#[test]
fn the_pane_survives_any_keys_at_any_size() {
    let _g = crate::app::motion_test_guard();
    let keys = [
        Key::Named(NamedKey::ArrowUp),
        Key::Named(NamedKey::ArrowDown),
        Key::Named(NamedKey::ArrowLeft),
        Key::Named(NamedKey::ArrowRight),
        Key::Named(NamedKey::Home),
        Key::Named(NamedKey::End),
        Key::Named(NamedKey::Backspace),
        Key::Named(NamedKey::Delete),
        Key::Named(NamedKey::Enter),
        Key::Named(NamedKey::Tab),
        Key::Named(NamedKey::Space),
        Key::Named(NamedKey::Escape),
        Key::Character("a".into()),
        Key::Character("é".into()),
        Key::Character("中".into()),
        Key::Character("👍".into()),
        Key::Character(",".into()),
        Key::Character("r".into()),
        Key::Character("f".into()),
        Key::Character("x".into()),
        Key::Character("q".into()),
    ];
    let sizes = [
        (1u16, 1u16),
        (2, 2),
        (5, 3),
        (8, 6),
        (12, 8),
        (20, 10),
        (30, 12),
        (40, 14),
        (60, 18),
        (80, 30),
        (139, 20),
        (140, 12),
        (150, 40),
        (300, 80),
    ];
    for seed in 1..150u64 {
        let mut rng = Rng(seed * 2654435761);
        let (mut p, _) = reading();
        let (cols, rows) = sizes[(rng.next() % sizes.len() as u64) as usize];
        for _ in 0..60 {
            let r = rng.next();
            match r % 14 {
                0 => {
                    p.paste("line one\nline two, Ann <a@b.co>\n");
                }
                10 => {
                    let rows = vec![
                        crate::inbox::wire::Thread {
                            id: "t1".into(),
                            subject: "Lunch".into(),
                            ..Default::default()
                        },
                        crate::inbox::wire::Thread {
                            id: "t9".into(),
                            subject: "Other".into(),
                            ..Default::default()
                        },
                    ];
                    let rows = if r % 3 == 0 {
                        vec![]
                    } else if r % 3 == 1 {
                        rows[1..].to_vec()
                    } else {
                        rows
                    };
                    p.take(crate::inbox::worker::Got::Page {
                        query: String::new(),
                        rows,
                        next: None,
                        more: r % 2 == 0,
                        whole: true,
                    });
                }
                11 => {
                    let mails = p
                        .reading
                        .as_ref()
                        .and_then(|r| r.mails.clone())
                        .unwrap_or_default();
                    let id = p
                        .reading
                        .as_ref()
                        .map(|r| r.thread.id.clone())
                        .unwrap_or_default();
                    p.take(crate::inbox::worker::Got::Opened(id, mails));
                }
                12 => p.take(match (r >> 8) % 4 {
                    0 => crate::inbox::worker::Got::Sent("a@b.co".into()),
                    1 => crate::inbox::worker::Got::Failed("network".into()),
                    2 => crate::inbox::worker::Got::Forwarded("a@b.co".into()),
                    _ => crate::inbox::worker::Got::Unread((r >> 16) % 50),
                }),
                13 if r % 50 == 0 => {
                    p.take(crate::inbox::worker::Got::Ended("invalid_grant".into()))
                }
                1 => {
                    let _ = p.click(
                        cols,
                        rows,
                        (r >> 8) as u16 % rows.max(1),
                        (r >> 24) as u16 % cols.max(1),
                    );
                }
                2 => {
                    p.scroll_wheel(((r >> 8) % 7) as i32 - 3, cols, rows);
                }
                _ => {
                    let k = &keys[((r >> 8) % keys.len() as u64) as usize];
                    let _ = p.press(k, cols, rows);
                }
            }
            let _ = p.cells(cols, rows);
            let _ = p.art(cols, rows, 2.0);
            let _ = p.pictures(cols, rows, 2.0);
            let _ = p.links(cols, rows);
        }
    }
}
