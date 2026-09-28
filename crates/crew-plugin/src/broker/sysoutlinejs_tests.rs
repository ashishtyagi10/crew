use crate::broker::sysoutline::render;

/// Functions three ways, a class with a constructor, a generic method and an
/// arrow-function property, the type-level definitions, and two that are not
/// definitions: a `{` in a line comment and in a string.
const TS: &str = r#"import { readFile } from "fs";

export const LIMIT = 10;
const internal = { a: 1 };

export function parse(text: string, strict = false): Node[] {
  const s = "function notInAString() {";
  return [];
}

export const handler = async (req: Request, res: Response): Promise<void> => {
  res.send("ok");
};

// function notInAComment() {
export class Store extends Base {
  private cache = new Map<string, string>();

  constructor(private root: string) {
    super();
  }

  async load<T>(key: string): Promise<T> {
    return JSON.parse(await readFile(key, "utf8"));
  }

  onChange = (e: Event) => {
    this.cache.clear();
  };
}

export interface Options {
  root: string;
}

export type Id = string | number;
"#;

#[test]
fn a_typescript_file_outlines_to_its_functions_classes_and_types() {
    assert_eq!(
        render("store.ts", TS).unwrap(),
        "store.ts \u{2014} 36 lines
   3  export const LIMIT
   6  export function parse(text, strict): Node[]
  11  export const handler = async (req, res): Promise<void> =>
  16  export class Store extends Base
  19    constructor(private root)
  23    async load<T>(key): Promise<T>
  27    onChange = (e) =>
  32  export interface Options
  36  export type Id"
    );
}

/// A constant holding a call that takes an arrow is not a function, and one
/// no one exports is the module's own business.
#[test]
fn a_constant_is_listed_when_it_is_a_function_or_exported() {
    let src = "const run = x => x * 2;\nconst sorted = items.map(() => 1);\nconst n = 3;\nexport const cfg = {\n  a: 1,\n};\n";
    assert_eq!(
        render("a.js", src).unwrap(),
        "a.js \u{2014} 6 lines\n   1  const run = x =>\n   4  export const cfg"
    );
}

#[test]
fn a_go_file_outlines_to_its_types_and_funcs_receivers_and_all() {
    let src = r#"package main

import "fmt"

type Server struct {
	addr string
}

type Handler func(w string) error

// func notInAComment() {
func (s *Server) Serve(addr string, n int) error {
	fmt.Println("func notInAString() {")
	return nil
}

func main() {
}
"#;
    assert_eq!(
        render("main.go", src).unwrap(),
        "main.go \u{2014} 18 lines
   5  type Server struct
   9  type Handler func(w string) error
  12  func (s *Server) Serve(addr string, n int) error
  17  func main()"
    );
}
