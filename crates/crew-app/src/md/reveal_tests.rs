//! The block the caret is in, drawn as its source.
use super::{block_at, Kind};

pub(crate) const DOC: &str = "\
# A heading

Some **bold** and *italic* and `code` in a paragraph that is long enough that
it has to wrap more than once at a narrow width.

- a list item
- another one, also long enough to wrap when the column count is small
  - and one nested under it

> a quotation, which the renderer draws a bar beside

| a | b |
|---|---|
| 1 | 2 |

```rust
fn main() { println!(\"hi\"); }
```

A [link](https://example.invalid) and a trailing line.
";

pub(crate) fn at(needle: &str) -> u32 {
    DOC.find(needle).expect(needle) as u32
}

fn revealed(needle: &str) -> &'static str {
    let r = block_at(DOC, at(needle)).expect(needle);
    &DOC[r.from as usize..r.to as usize]
}

#[test]
fn a_heading_reveals_its_hashes() {
    assert_eq!(revealed("heading"), "# A heading");
}

#[test]
fn a_paragraph_reveals_all_its_lines_and_nothing_else() {
    assert_eq!(
        revealed("italic"),
        "Some **bold** and *italic* and `code` in a paragraph that is long enough that\n\
         it has to wrap more than once at a narrow width."
    );
}

/// A tight item has no paragraph of its own; it is the leaf — but not the list under it.
#[test]
fn a_list_item_reveals_its_marker_and_not_the_items_nested_under_it() {
    assert_eq!(revealed("a list item"), "- a list item");
    assert_eq!(
        revealed("another one"),
        "- another one, also long enough to wrap when the column count is small"
    );
    assert_eq!(revealed("and one nested"), "  - and one nested under it");
}

#[test]
fn a_quote_reveals_its_bar() {
    assert_eq!(
        revealed("a quotation"),
        "> a quotation, which the renderer draws a bar beside"
    );
}

#[test]
fn a_table_reveals_whole_and_a_fence_with_its_fences() {
    assert_eq!(revealed("| 1"), "| a | b |\n|---|---|\n| 1 | 2 |");
    let r = block_at(DOC, at("fn main")).unwrap();
    assert_eq!(r.kind, Kind::Fence);
    assert_eq!(
        &DOC[r.from as usize..r.to as usize],
        "```rust\nfn main() { println!(\"hi\"); }\n```"
    );
}

/// The end of a line is in its block: that is where a caret appends to it.
#[test]
fn the_end_of_the_last_line_is_inside_the_block() {
    let end = at("# A heading") + "# A heading".len() as u32;
    let r = block_at(DOC, end).unwrap();
    assert_eq!((r.from, r.to), (0, end));
}

#[test]
fn a_blank_line_between_blocks_reveals_nothing() {
    assert_eq!(block_at(DOC, at("\n\nSome") + 1), None);
}
