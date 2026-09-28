use crate::broker::sysoutline::render;

/// A class, its methods, a def nested in one, and two things that look like
/// they end or begin a definition: a `def` in the module docstring, and a
/// signature over four lines whose `)` sits at the method's own indent.
const PY: &str = r#""""A module docstring.

def not_a_function_in_a_docstring():
"""
import os


class Store:
    """Keeps things."""

    def __init__(self, root: str = "."):
        self.root = root

    async def load(
        self,
        key: str,
    ) -> bytes:
        def decode(raw):
            return raw
        return decode(b"")


def main(argv=None) -> int:
    return 0
"#;

#[test]
fn a_python_file_outlines_by_indentation() {
    assert_eq!(
        render("store.py", PY).unwrap(),
        "store.py \u{2014} 24 lines
   8  class Store
  11    def __init__(self, root)
  14    async def load(self, key) -> bytes
  18      def decode(raw)
  23  def main(argv) -> int"
    );
}

#[test]
fn a_markdown_file_outlines_to_its_headings_and_not_its_code() {
    let md = "# Title\n\nIntro.\n\n## Build\n\n```sh\n# not a heading\n```\n\n### Steps\n## Test\n    # indented code, not a heading\n";
    assert_eq!(
        render("notes.md", md).unwrap(),
        "notes.md \u{2014} 13 lines
   1  # Title
   5    ## Build
  11      ### Steps
  12    ## Test"
    );
}

/// A README that starts at `##` is not indented a level for the `#` it
/// does not have.
#[test]
fn the_shallowest_heading_present_is_the_left_edge() {
    let md = "## One\n### One.a\n## Two\n";
    assert_eq!(
        render("README.md", md).unwrap(),
        "README.md \u{2014} 3 lines\n   1  ## One\n   2    ### One.a\n   3  ## Two"
    );
}

/// A line that starts with inline code in four backticks opens no fence,
/// and a fence closes only on a run as long as the one that opened it.
#[test]
fn inline_code_opens_no_fence_and_a_fence_closes_on_its_own_length() {
    let md = "# A\n  ```` ```diff ```` fences\n## B\n````md\n```\n# in the fence\n````\n## C\n";
    assert_eq!(
        render("x.md", md).unwrap(),
        "x.md \u{2014} 8 lines\n   1  # A\n   3    ## B\n   8    ## C"
    );
}
