use super::{parse, Kid, Node};

fn shape(n: &Node) -> String {
    let kids: Vec<String> = n
        .kids
        .iter()
        .map(|k| match k {
            Kid::El(e) => shape(e),
            Kid::Text(t) => format!("'{t}'"),
        })
        .collect();
    format!("{}({})", n.tag, kids.join(","))
}

#[test]
fn unclosed_paragraphs_items_and_cells_nest_as_they_look() {
    let t = parse("<p>a<p>b<ul><li>x<li>y</ul><table><tr><td>1<td>2<tr><td>3</table></b>c");
    assert_eq!(
        shape(&t),
        "#root(p('a'),p('b'),ul(li('x'),li('y')),table(tr(td('1'),td('2')),tr(td('3'))),'c')"
    );
}

#[test]
fn ten_thousand_open_divs_stop_nesting() {
    let t = parse(&"<div>".repeat(10_000));
    let mut depth = 0;
    let mut n = &t;
    while let Some(Kid::El(k)) = n.kids.first() {
        depth += 1;
        n = k;
    }
    assert!(depth <= super::DEEP, "{depth}");
}
