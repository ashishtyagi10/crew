use super::*;

fn sym(name: &str) -> Symbol {
    Symbol {
        name: name.into(),
        kind: "struct",
        uri: "file:///p/a.rs".into(),
        line: Some(0),
    }
}

#[test]
fn both_reply_shapes_parse() {
    let info = json!([{"name": "ToolBudget", "kind": 23, "containerName": "m",
        "location": {"uri": "file:///p/a.rs",
            "range": {"start": {"line": 6, "character": 0}, "end": {"line": 6, "character": 4}}}}]);
    let s = Symbol::parse_many(&info);
    assert_eq!(
        s,
        [Symbol {
            name: "ToolBudget".into(),
            kind: "struct",
            uri: "file:///p/a.rs".into(),
            line: Some(6)
        }]
    );
    // WorkspaceSymbol: a location with only a uri, no range.
    let ws = json!([{"name": "f", "kind": 12, "location": {"uri": "file:///p/b.rs"}}, {"junk": 1}]);
    let s = Symbol::parse_many(&ws);
    assert_eq!((s.len(), s[0].kind, s[0].line), (1, "function", None));
    assert!(Symbol::parse_many(&Value::Null).is_empty());
}

#[test]
fn kinds_have_words_and_unknowns_fall_back() {
    assert_eq!(kind_label(5), "class");
    assert_eq!(kind_label(26), "type parameter");
    assert_eq!(kind_label(0), "symbol");
    assert_eq!(kind_label(99), "symbol");
}

#[test]
fn exact_then_prefix_then_the_servers_order() {
    let got = rank(
        "Tool",
        vec![
            sym("MyTool"),
            sym("ToolBudget"),
            sym("Tool"),
            sym("tooling"),
            sym("AToolB"),
        ],
    );
    let names: Vec<&str> = got.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["Tool", "ToolBudget", "tooling", "MyTool", "AToolB"]);
}
