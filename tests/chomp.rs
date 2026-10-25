// Literal values copied from the upstream Perl test, not math constants.
#![allow(clippy::approx_constant)]

mod common;
use std::collections::HashMap;

use common::*;
use template_toolkit::{
    constants::*, context::BlockDef, provider::IncludePathItem, value::Value, Template,
};

fn od(s: &str) -> String {
    s.chars()
        .map(|c| {
            let o = c as u32;
            if !(32..=127).contains(&o) {
                format!("\\0{:o}", o)
            } else {
                c.to_string()
            }
        })
        .collect()
}

#[test]
fn chomp_constants() {
    assert_eq!(CHOMP_NONE, 0);
    assert_eq!(CHOMP_ONE, 1);
    assert_eq!(CHOMP_ALL, 1);
    assert_eq!(CHOMP_COLLAPSE, 2);
    assert_eq!(CHOMP_GREEDY, 3);
}

fn blocks() -> HashMap<String, BlockDef> {
    let b: Vec<(&str, &str)> = vec![
        ("foo", "\n[% foo %]\n"),
        ("bar", "\n[%- bar -%]\n"),
        ("baz", "\n[%+ baz +%]\n"),
        ("ding", "!\n\n[%~ ding ~%]\n\n!"),
        ("dong", "!\n\n[%= dong =%]\n\n!"),
        ("dang", "Hello[%# blah blah blah -%]\n!"),
        ("winsux1", "[% ding -%]\r\n[% dong %]"),
        ("winsux2", "[% ding -%]\r\n\r\n[% dong %]"),
        ("winsux3", "[% ding %]\r\n[%- dong %]"),
        ("winsux4", "[% ding %]\r\n\r\n[%- dong %]"),
    ];
    b.into_iter()
        .map(|(k, v)| (k.to_string(), BlockDef::from(v)))
        .collect()
}

fn vars_() -> Vars {
    vars(vec![
        ("foo", Value::Float(3.14)),
        ("bar", Value::Float(2.718)),
        ("baz", Value::Float(1.618)),
        ("ding", "Hello".into()),
        ("dong", "World".into()),
    ])
}

fn run(tt: &Template, name: &str) -> String {
    tt.process_file(name, Some(vars_()))
        .unwrap_or_else(|e| panic!("{}: {}", name, e))
}

#[test]
fn chomp() {
    let tt2 = Template::builder()
        .blocks(blocks())
        .include_path(vec![IncludePathItem::Path(
            "corpora/Template-Toolkit-3.009/t/test/lib".into(),
        )])
        .build()
        .unwrap();
    assert_eq!(run(&tt2, "foo"), "\n3.14\n");
    assert_eq!(run(&tt2, "bar"), "2.718");
    assert_eq!(run(&tt2, "baz"), "\n1.618\n");
    assert_eq!(run(&tt2, "ding"), "!Hello!");
    assert_eq!(run(&tt2, "dong"), "! World !");
    assert_eq!(run(&tt2, "dang"), "Hello!");
    assert_eq!(od(&run(&tt2, "winsux1")), "HelloWorld");
    assert_eq!(od(&run(&tt2, "winsux2")), "Hello\\015\\012World");
    assert_eq!(od(&run(&tt2, "winsux3")), "HelloWorld");
    assert_eq!(od(&run(&tt2, "winsux4")), "Hello\\015\\012World");
    assert_eq!(run(&tt2, "dos_newlines"), "HelloWorld");

    let pre = Template::builder()
        .pre_chomp(1)
        .blocks(blocks())
        .build()
        .unwrap();
    assert_eq!(run(&pre, "foo"), "3.14\n");
    assert_eq!(run(&pre, "bar"), "2.718");
    assert_eq!(run(&pre, "baz"), "\n1.618\n");
    assert_eq!(run(&pre, "ding"), "!Hello!");
    assert_eq!(run(&pre, "dong"), "! World !");

    let post = Template::builder()
        .post_chomp(1)
        .blocks(blocks())
        .build()
        .unwrap();
    assert_eq!(run(&post, "foo"), "\n3.14");
    assert_eq!(run(&post, "bar"), "2.718");
    assert_eq!(run(&post, "baz"), "\n1.618\n");
    assert_eq!(run(&post, "ding"), "!Hello!");
    assert_eq!(run(&post, "dong"), "! World !");
}

#[test]
fn chomp_corpus() {
    let b = |pre: usize, post: usize| {
        Template::builder()
            .pre_chomp(pre)
            .post_chomp(post)
            .build()
            .unwrap()
    };
    let tt = vec![
        ("tt_pre_none", b(CHOMP_NONE, 0)),
        ("tt_pre_one", b(CHOMP_ONE, 0)),
        ("tt_pre_all", b(CHOMP_ALL, 0)),
        ("tt_pre_coll", b(CHOMP_COLLAPSE, 0)),
        ("tt_post_none", b(0, CHOMP_NONE)),
        ("tt_post_one", b(0, CHOMP_ONE)),
        ("tt_post_all", b(0, CHOMP_ALL)),
        ("tt_post_coll", b(0, CHOMP_COLLAPSE)),
    ];
    test_expect(&tt_test("chomp"), tt, vars(vec![]));
}
