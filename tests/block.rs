mod common;
use std::{collections::HashMap, rc::Rc};

use common::*;
use template_toolkit::{
    constants::CHOMP_ALL, context::BlockDef, provider::IncludePathItem, value::Value, Template,
};

fn callsign() -> Vars {
    let w = [
        "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel", "india",
        "juliet", "kilo", "lima", "mike", "november", "oscar", "papa", "quebec", "romeo", "sierra",
        "tango", "umbrella", "victor", "whisky", "x-ray", "yankee", "zulu",
    ];
    ('a'..='z')
        .zip(w)
        .map(|(k, v)| (k.to_string(), Value::from(v)))
        .collect()
}

#[test]
fn block() {
    let mut blocks: HashMap<String, BlockDef> = HashMap::new();
    blocks.insert(
        "header".into(),
        "<html><head><title>[% title %]</title></head><body>".into(),
    );
    blocks.insert("footer".into(), "</body></html>".into());
    blocks.insert(
        "block_a".into(),
        BlockDef::Closure(Rc::new(|_| Ok("this is block a".to_string()))),
    );
    blocks.insert(
        "block_b".into(),
        BlockDef::Closure(Rc::new(|_| Ok("this is block b".to_string()))),
    );
    let tt = Template::builder()
        .include_path(vec![IncludePathItem::Path(
            "Template-Toolkit-3.009/t/test/lib".into(),
        )])
        .post_chomp(CHOMP_ALL)
        .blocks(blocks)
        .build()
        .unwrap();
    test_expect(
        include_str!("corpus/block.tt"),
        vec![("default", tt)],
        callsign(),
    );
}
