//! The open thread's links where they are drawn, so both ways of reaching
//! one land on its real address (goal 2026-10-20-mail-reader, item 7): a
//! Cmd+click on its words, and `Cmd+E`'s hint mode, which reads only what is
//! on screen and would otherwise see "Track your order" and nothing to open.
use super::{reader, InboxPane};

impl InboxPane {
    /// Every link on screen, where its words start: `(row, col, address)`.
    pub(crate) fn links(&self, cols: u16, rows: u16) -> Vec<(u16, u16, String)> {
        let Some(r) = &self.reading else {
            return Vec::new();
        };
        let laid = r.laid(reader::text_width(cols));
        let room = rows.saturating_sub(1) as usize;
        let mut out = Vec::new();
        for (row, line) in laid.lines.iter().skip(r.scroll).take(room).enumerate() {
            let mut col: u16 = 1;
            let mut prev: Option<&str> = None;
            for cell in line {
                let w = crate::chatwidth::char_w(cell.c) as u16;
                let link = cell.link.as_deref();
                if let Some(url) = link.filter(|&u| prev != Some(u)) {
                    out.push((row as u16, col, url.to_string()));
                }
                prev = link;
                col += w;
            }
        }
        out
    }

    /// The address under a click at `(row, col)`, if a link is drawn there.
    pub(crate) fn link_at(&self, cols: u16, rows: u16, row: u16, col: u16) -> Option<String> {
        let r = self.reading.as_ref()?;
        if row >= rows.saturating_sub(1) || col == 0 {
            return None;
        }
        let laid = r.laid(reader::text_width(cols));
        let line = laid.lines.get(r.scroll + row as usize)?;
        let cell = crate::chatplace::cell_at_col(line, col - 1)?;
        cell.link.as_deref().map(str::to_string)
    }
}

#[cfg(test)]
mod tests {
    use crate::inbox::api::Mail;
    use crate::inbox::rich::Rich;
    use crate::inbox::wire::Thread;
    use crate::inbox::{InboxPane, Reading, Stage};

    #[test]
    fn a_links_words_lead_to_its_real_address_by_hint_and_by_click() {
        let (tx, _) = std::sync::mpsc::channel();
        let (_, rx) = std::sync::mpsc::channel();
        let mut p = InboxPane::with(Stage::Ready(crate::inbox::worker::Worker { tx, rx }));
        let rich = Rich::of_html(
            "<p>See <a href='https://x.io/track?id=1'>Track it</a> now</p>",
            None,
        );
        p.reading = Some(Reading {
            mails: Some(vec![Mail {
                rich,
                ..Mail::default()
            }]),
            ..Reading::new(Thread::default())
        });
        let links = p.links(60, 20);
        assert_eq!(links.len(), 1, "{links:?}");
        let (row, col, url) = links[0].clone();
        assert_eq!(url, "https://x.io/track?id=1");
        assert_eq!(
            p.link_at(60, 20, row, col + 3).as_deref(),
            Some(url.as_str())
        );
        assert_eq!(p.link_at(60, 20, row, col - 2), None, "the words before it");
    }
}
