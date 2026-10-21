//! Tables, of which mail has two kinds. A data table (a receipt's items, a
//! schedule) is rows of short cells and becomes a markdown table. A layout
//! table is how a newsletter places its columns, banners and buttons in
//! every mail client; drawn as a grid it is nonsense columns, so its cells
//! are read one after another as blocks instead.
use super::tree::{Kid, Node};

/// Cell content that says the table is for layout, not data.
const LAYOUT_INSIDE: &[&str] = &[
    "table",
    "img",
    "ul",
    "ol",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "blockquote",
    "pre",
    "hr",
];

/// The table's rows (through `thead`/`tbody`/`tfoot`, not into nested
/// tables) as their cells.
pub(crate) fn rows(t: &Node) -> Vec<Vec<&Node>> {
    let mut out = Vec::new();
    for k in &t.kids {
        let Kid::El(n) = k else { continue };
        match n.tag.as_str() {
            "tr" => out.push(cells(n)),
            "thead" | "tbody" | "tfoot" => out.extend(rows(n)),
            _ => {}
        }
    }
    out
}

fn cells(tr: &Node) -> Vec<&Node> {
    tr.kids
        .iter()
        .filter_map(|k| match k {
            Kid::El(n) if n.tag == "td" || n.tag == "th" => Some(n),
            _ => None,
        })
        .collect()
}

/// Whether `t` holds data rather than placing things.
pub(crate) fn is_data(t: &Node) -> bool {
    if t.attr("role")
        .is_some_and(|r| r.eq_ignore_ascii_case("presentation"))
    {
        return false;
    }
    let rows = rows(t);
    let width = rows.iter().map(Vec::len).max().unwrap_or(0);
    if rows.len() < 2 || width < 2 {
        return false;
    }
    let all: Vec<&&Node> = rows.iter().flatten().collect();
    let empty = all.iter().filter(|c| c.text().trim().is_empty()).count();
    let busy = all.iter().any(|c| {
        c.any(&|n| LAYOUT_INSIDE.contains(&n.tag.as_str())) || c.text().trim().chars().count() > 120
    });
    !busy && empty * 2 <= all.len()
}

/// A data table as markdown: the first row is the header (GFM needs one),
/// every row padded to the widest.
/// `cells` as a markdown table; without a `head` row (`th` cells), an empty
/// header, which the renderer leaves out.
pub(crate) fn markdown(cells: &[Vec<String>], head: bool) -> String {
    let width = cells.iter().map(Vec::len).max().unwrap_or(0);
    let row = |r: &[String]| {
        let mut parts: Vec<String> = r.iter().map(|c| c.replace('\n', " ")).collect();
        parts.resize(width, String::new());
        format!("| {} |", parts.join(" | "))
    };
    let (top, body) = match head {
        true => (row(&cells[0]), &cells[1..]),
        false => (format!("|{}", "  |".repeat(width)), cells),
    };
    let mut out = vec![top, format!("|{}", " --- |".repeat(width))];
    out.extend(body.iter().map(|r| row(r)));
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::super::tree::{parse, Kid, Node};
    use super::*;

    fn table(html: &str) -> Node {
        match parse(html).kids.into_iter().next() {
            Some(Kid::El(n)) => n,
            _ => panic!("no table"),
        }
    }

    #[test]
    fn a_receipt_is_data_and_a_newsletter_grid_is_layout() {
        assert!(is_data(&table(
            "<table><tr><th>Item<th>Price<tr><td>Tea<td>$4<tr><td>Cake<td>$6</table>"
        )));
        assert!(!is_data(&table(
            "<table><tr><td><img src=a.png><td>Hi<tr><td>x<td>y</table>"
        )));
        assert!(!is_data(&table(
            "<table role=presentation><tr><td>a<td>b<tr><td>c<td>d</table>"
        )));
        assert!(!is_data(&table("<table><tr><td>only one row<td>x</table>")));
        assert!(!is_data(&table(
            "<table><tr><td><table><tr><td>a</table><td>b<tr><td>c<td>d</table>"
        )));
    }

    #[test]
    fn rows_are_padded_under_a_header() {
        let c = |s: &[&str]| s.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(
            markdown(&[c(&["a", "b"]), c(&["1"])], true),
            "| a | b |\n| --- | --- |\n| 1 |  |"
        );
        assert_eq!(
            markdown(&[c(&["Tea", "$4"]), c(&["Cake", "$6"])], false),
            "|  |  |\n| --- | --- |\n| Tea | $4 |\n| Cake | $6 |",
            "no th row: an empty header, every row data"
        );
    }

    #[test]
    fn an_empty_header_is_left_out_with_its_rule() {
        let lines =
            crate::md::render_chat("|  |  |\n| --- | --- |\n| Tea | $4 |\n| Cake | $6 |", 40);
        let text: Vec<String> = lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.text.as_str()).collect::<String>())
            .collect();
        assert_eq!(
            text.len(),
            2,
            "two data rows and nothing over them: {text:?}"
        );
        assert!(
            text[0].contains("Tea") && !text.iter().any(|l| l.contains('\u{2500}')),
            "{text:?}"
        );
    }
}
