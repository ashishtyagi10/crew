//! Simple regex-free syntax highlighter for common file types.
//! Produces colored Spans for each line based on file extension.

mod builtins;
mod colors;
mod highlighter;
mod json;
mod keywords;
mod keywords_data;
mod language;
mod markdown;
mod scanners;

pub use highlighter::highlight_line;
pub use language::Language;
