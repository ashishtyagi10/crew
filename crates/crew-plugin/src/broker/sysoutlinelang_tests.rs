use crate::broker::sysoutline::render;

/// Every kind the outline names, and three that only look like one: a `fn`
/// at the start of a line inside a string, one in a block comment, and a
/// `{` in a line comment and in a char literal, either of which, counted,
/// would bury everything after it a block deeper.
const RUST: &str = r#"//! A fixture.
use std::fmt;

const TASK_CAP: usize = 4_000;

const HELP: &str = "
fn not_a_function_in_a_string() {
";

// fn not_a_function_in_a_comment() {
/*
fn nor_in_a_block_comment() {}
*/

/// Says `fn frame` in a doc comment.
pub fn frame(
    env: &Envelope,
    peers: &[String],
) -> String {
    let open = '{';
    format!("{open}")
}

pub struct Item {
    pub line: usize,
}

impl Item {
    pub fn new(line: usize) -> Self {
        Self { line }
    }

    pub async fn load<'a>(&'a self, path: &str) -> Result<(), String> {
        Ok(())
    }
}

mod inner {
    pub(crate) enum Kind {
        A,
    }
}
"#;

#[test]
fn a_rust_file_outlines_to_its_definitions_at_their_lines() {
    assert_eq!(
        render("fixture.rs", RUST).unwrap(),
        "fixture.rs \u{2014} 42 lines
   4  const TASK_CAP
   6  const HELP
  16  pub fn frame(env, peers) -> String
  24  pub struct Item
  28  impl Item
  29    pub fn new(line) -> Self
  33    pub async fn load<'a>(&'a self, path) -> Result<(), String>
  38  mod inner
  39    pub(crate) enum Kind"
    );
}

/// A trait's methods go under it, a `where` clause does not end a head, an
/// out-of-line `mod x;` opens nothing, and a `fn` or `impl` inside a
/// function body is that function's business.
#[test]
fn traits_where_clauses_and_bodies_nest_as_they_are_written() {
    let src = "#[cfg(test)]
mod tests;

pub(crate) trait Runner: Send {
    fn hint(&self) -> String;
    async fn call(&self, tool: &str) -> Result<String, String>;
}

impl<T> Runner for Wrap<T>
where
    T: Runner,
{
    fn hint(&self) -> String {
        fn helper() {}
        struct Local;
        impl Local {}
        String::new()
    }
}

pub const fn size() -> usize { 4 }
pub(crate) static mut COUNT: u32 = 0;
type Result<T> = std::result::Result<T, Error>;
const _: () = assert!(true);
macro_rules! log {
    ($x:expr) => {};
}
pub struct Id(pub u64);
";
    assert_eq!(
        render("lib.rs", src).unwrap(),
        "lib.rs \u{2014} 28 lines
   2  mod tests
   4  pub(crate) trait Runner: Send
   5    fn hint(&self) -> String
   6    async fn call(&self, tool) -> Result<String, String>
   9  impl<T> Runner for Wrap<T>
  13    fn hint(&self) -> String
  21  pub const fn size() -> usize
  22  pub(crate) static mut COUNT
  23  type Result<T>
  25  macro_rules! log
  28  pub struct Id"
    );
}
