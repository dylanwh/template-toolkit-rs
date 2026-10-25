// Literal values copied from the upstream Perl test, not math constants.
#![allow(clippy::approx_constant)]

mod common;
use std::{collections::HashMap, rc::Rc};

use common::*;
use template_toolkit::{
    document::Document,
    parser::ast::{Block, SpannedStmt, Stmt},
    provider::IncludePathItem,
    value::Value,
    Template,
};

fn text_block(s: &str) -> Block {
    Block {
        stmts: vec![SpannedStmt {
            line: None,
            file: None,
            stmt: Stmt::Text(s.to_string()),
        }],
    }
}

#[test]
fn document() {
    // Upstream builds a Template::Document from Perl coderefs (BLOCK, DEFBLOCKS)
    // and calls them directly; the Rust Document wraps an AST Block instead, so
    // the direct coderef invocations (`&{ $doc->block }` etc.) are not portable.
    // The metadata accessors and DEFBLOCKS are checked through the Rust API.
    let mut defblocks = HashMap::new();
    defblocks.insert("foo".to_string(), text_block("the foo block"));
    defblocks.insert("bar".to_string(), text_block("the bar block"));
    let mut doc = Document::from_block(text_block("some output"), None);
    doc.defblocks = defblocks;
    doc.set_meta("author", "Andy Wardley".into());
    doc.set_meta("version", Value::Float(3.14));
    assert_eq!(
        doc.get_meta("author").unwrap().to_tt_string(),
        "Andy Wardley"
    );
    assert_eq!(doc.get_meta("version").unwrap().to_number(), 3.14);
    assert!(doc.defblocks.contains_key("foo") && doc.defblocks.contains_key("bar"));

    let dir = "corpora/Template-Toolkit-3.009/t/test";
    let tproc = Template::builder()
        .include_path(vec![IncludePathItem::Path(format!("{dir}/src"))])
        .build()
        .unwrap();
    let mydoc = Value::Object(Rc::new(doc));
    test_expect(
        &tt_test("document"),
        vec![("default", tproc)],
        vars(vec![("mydoc", mydoc)]),
    );
}
