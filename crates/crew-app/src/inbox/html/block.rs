//! Laying the open paragraph into the document: `> ` per quote level, a list
//! item's marker or its hanging indent, a heading's `#`s, a code fence.
use super::out::{List, Out};

impl Out {
    pub(crate) fn list_open(&mut self, ordered: bool, start: u32) {
        self.block();
        self.lists.push(List {
            ordered,
            next: start,
        });
    }

    pub(crate) fn list_close(&mut self) {
        self.block();
        self.lists.pop();
        if self.lists.is_empty() {
            self.gap();
        }
    }

    pub(crate) fn item(&mut self) {
        self.block();
        self.item = !self.lists.is_empty();
    }

    pub(crate) fn hr(&mut self) {
        self.block();
        let q = self.quote_prefix();
        self.doc.push_str(&format!("{q}---\n"));
        self.gap();
    }

    /// End the paragraph: lay it into the document.
    pub(crate) fn block(&mut self) {
        let text = std::mem::take(&mut self.line);
        self.gen += 1;
        let at = self.doc.len();
        match self.pre {
            true => self.fence(&text),
            false if !text.trim().is_empty() => self.para(text.trim_end()),
            false => {}
        }
        self.head = 0;
        // Small print (12px or less) the mail ends with is its footer.
        if self.doc.len() > at {
            let small = self.line_size > 0.0 && self.line_size <= 12.5;
            self.small_from = small.then(|| self.small_from.unwrap_or(at));
        }
    }

    fn para(&mut self, text: &str) {
        let q = self.quote_prefix();
        let indent: usize = self
            .lists
            .iter()
            .rev()
            .skip(1)
            .map(|l| marker_w(l.ordered))
            .sum();
        let mut first = true;
        for line in text.split('\n').map(str::trim_start) {
            let lead = match (first && self.item, self.lists.last_mut()) {
                (true, Some(l)) if l.ordered => {
                    l.next += 1;
                    format!("{}{}. ", " ".repeat(indent), l.next - 1)
                }
                (true, Some(_)) => format!("{}- ", " ".repeat(indent)),
                (_, Some(l)) => " ".repeat(indent + marker_w(l.ordered)),
                (_, None) => String::new(),
            };
            let head = match (first, self.head) {
                (true, n @ 1..=6) => format!("{} ", "#".repeat(usize::from(n))),
                _ => String::new(),
            };
            self.doc
                .push_str(&format!("{q}{lead}{head}{}\n", start_safe(line)));
            first = false;
        }
        self.item = false;
        if self.lists.is_empty() {
            self.gap();
        }
    }

    fn fence(&mut self, text: &str) {
        let q = self.quote_prefix();
        let run = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
        let ticks = "`".repeat(run.max(2) + 1);
        self.doc.push_str(&format!("{q}{ticks}\n"));
        for line in text.trim_matches('\n').split('\n') {
            self.doc.push_str(&format!("{q}{line}\n"));
        }
        self.doc.push_str(&format!("{q}{ticks}\n"));
        self.gap();
    }

    /// A blank line, inside the quote when there is one.
    pub(crate) fn gap(&mut self) {
        let q = self.quote_prefix();
        self.doc.push_str(q.trim_end());
        self.doc.push('\n');
    }

    fn quote_prefix(&self) -> String {
        "> ".repeat(self.quote)
    }

    /// The markdown, finished.
    /// The document, where its quoted history starts, and where the small
    /// print it ends with starts.
    pub(crate) fn done(mut self) -> (String, Option<usize>, Option<usize>) {
        self.pre = false;
        self.block();
        (self.doc, self.split, self.small_from)
    }

    /// Lines the walk built whole (a table), at the current quote depth.
    pub(crate) fn lines(&mut self, md: &str) {
        self.block();
        let q = self.quote_prefix();
        for line in md.lines() {
            self.doc.push_str(&format!("{q}{line}\n"));
        }
        self.gap();
    }
}

fn marker_w(ordered: bool) -> usize {
    if ordered {
        3
    } else {
        2
    }
}

/// A line whose start would read as markdown (`- `, `1.`, `===`) escaped.
pub(crate) fn start_safe(line: &str) -> String {
    if line.starts_with(['-', '+', '=']) {
        return format!("\\{line}");
    }
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    match line[digits..].chars().next() {
        Some(c @ ('.' | ')')) if digits > 0 => {
            format!("{}\\{c}{}", &line[..digits], &line[digits + 1..])
        }
        _ => line.to_string(),
    }
}
