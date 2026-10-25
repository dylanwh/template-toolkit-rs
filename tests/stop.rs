mod common;
use std::{collections::HashMap, rc::Rc};

use common::*;
use template_toolkit::{
    context::BlockDef,
    error::{ControlFlow, TemplateException},
    value::Value,
    Template,
};

fn blocks() -> HashMap<String, BlockDef> {
    let mut b: HashMap<String, BlockDef> = HashMap::new();
    b.insert(
        "header".into(),
        BlockDef::Closure(Rc::new(|_| Ok("This is the header\n".to_string()))),
    );
    b.insert(
        "footer".into(),
        BlockDef::Closure(Rc::new(|_| Ok("This is the footer\n".to_string()))),
    );
    b.insert(
        "halt1".into(),
        BlockDef::Closure(Rc::new(|_| {
            Err(ControlFlow::Exception(TemplateException::new(
                "stop",
                "big error",
            )))
        })),
    );
    b
}

#[test]
fn stop() {
    let ttbare = Template::builder().blocks(blocks()).build().unwrap();
    let ttwrap = Template::builder()
        .pre_process(vec!["header".into()])
        .post_process(vec!["footer".into()])
        .blocks(blocks())
        .build()
        .unwrap();
    let halt = Value::function(|_, _| {
        Err(ControlFlow::Exception(TemplateException::new(
            "stop",
            "big error",
        )))
    });
    test_expect(
        &tt_test("stop"),
        vec![("bare", ttbare), ("wrapped", ttwrap)],
        vars(vec![("halt", halt)]),
    );
}
