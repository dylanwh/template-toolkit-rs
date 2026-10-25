mod common;
use std::{cell::Cell, collections::HashMap, rc::Rc};

use common::*;
use template_toolkit::{
    parser::{ConstantsNamespace, Parser},
    value::Value,
    Template,
};

fn make_constants() -> HashMap<String, Value> {
    let n = Rc::new(Cell::new(0i64));
    let mut m = HashMap::new();
    m.insert(
        "author".to_string(),
        Value::Str("Andy 'Da Man' Wardley".into()),
    );
    m.insert("single".to_string(), Value::Str("foo'bar".into()));
    m.insert("double".to_string(), Value::Str("foo'bar".into()));
    m.insert("joint".to_string(), Value::Str(", ".into()));
    m.insert(
        "col".to_string(),
        hash(vec![("back", "#ffffff".into()), ("text", "#000000".into())]),
    );
    m.insert(
        "counter".to_string(),
        Value::function(move |_, _| {
            let v = n.get();
            n.set(v + 1);
            Ok(Value::Int(v))
        }),
    );
    m
}

// Upstream checks the generated Perl source text of the parsed document
// (regexes on $parsed->{BLOCK}) and Namespace->ident() code strings; those are
// Perl-codegen specific.  The equivalent here: the constants are folded into
// the AST at parse time while col.user stays a runtime variable.
#[test]
fn constants_folding() {
    let ns = ConstantsNamespace::new(make_constants());
    let mut p = Parser::builder()
        .namespaces([("const".to_string(), ns)].into())
        .build()
        .unwrap();
    let parsed = p
        .parse(
            "hello [% const.author %]\n[% \"back is $const.col.back\" %] and text is [% const.col.text %]\nbut a col is still a [% col.user %]\n",
            None,
        )
        .unwrap();
    let dbg = format!("{:?}", parsed);
    assert!(
        dbg.contains("Andy 'Da Man' Wardley"),
        "author folded: {}",
        dbg
    );
    assert!(dbg.contains("#ffffff"), "col.back folded: {}", dbg);
    assert!(dbg.contains("#000000"), "col.text folded: {}", dbg);
    assert!(!dbg.contains("\"const\""), "const unfolded: {}", dbg);
    assert!(dbg.contains("\"user\""), "col.user unfolded: {}", dbg);
}

#[test]
fn constants_corpus() {
    let ns = ConstantsNamespace::new(make_constants());
    let tt1 = Template::builder()
        .namespace([("const".to_string(), ns)].into())
        .build()
        .unwrap();
    let const2 = || -> HashMap<String, Value> {
        [
            ("author".to_string(), Value::from("abw")),
            ("joint".to_string(), Value::from(" is the new ")),
            (
                "col".to_string(),
                hash(vec![("back", "orange".into()), ("text", "black".into())]),
            ),
            ("fave".to_string(), Value::from("back")),
        ]
        .into()
    };
    let tt2 = Template::builder().constants(const2()).build().unwrap();
    let tt3 = Template::builder()
        .constants(const2())
        .constants_namespace("const".to_string())
        .build()
        .unwrap();
    let v = vars(vec![
        (
            "col",
            hash(vec![("user", "red".into()), ("luza", "blue".into())]),
        ),
        ("constants", Value::hash(make_constants())),
    ]);
    test_expect(
        include_str!("corpus/constants.tt"),
        vec![("tt1", tt1), ("tt2", tt2), ("tt3", tt3)],
        v,
    );
}
