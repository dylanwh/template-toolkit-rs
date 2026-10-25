mod common;
use std::{collections::HashMap, rc::Rc};

use common::*;
use template_toolkit::{
    context::BlockDef, provider::IncludePathItem, service::ErrorSpec, Template,
};

#[test]
fn service() {
    let dir = format!(
        "{}/corpora/Template-Toolkit-3.009/t/test",
        env!("CARGO_MANIFEST_DIR")
    );
    let ip = || {
        vec![
            IncludePathItem::Path(format!("{}/src", dir)),
            IncludePathItem::Path(format!("{}/lib", dir)),
        ]
    };
    let blocks = || {
        let mut b: HashMap<String, BlockDef> = HashMap::new();
        b.insert(
            "demo".into(),
            BlockDef::Closure(Rc::new(|_| Ok("This is a demo".to_string()))),
        );
        b.insert(
            "astext".into(),
            "Another template block, a is '[% a %]'".into(),
        );
        b
    };
    let err_map = || {
        ErrorSpec::Map(HashMap::from([
            ("barf".to_string(), "barfed".to_string()),
            ("default".to_string(), "error".to_string()),
        ]))
    };

    let tt1 = Template::builder()
        .include_path(ip())
        .pre_process(vec!["config".into(), "header".into()])
        .post_process(vec!["footer".into()])
        .blocks(blocks())
        .error(err_map())
        .build()
        .unwrap();
    let tt2 = Template::builder()
        .include_path(ip())
        .pre_process(vec!["config".into(), "header".into()])
        .post_process(vec!["footer".into()])
        .blocks(blocks())
        .error(err_map())
        .auto_reset(false)
        .build()
        .unwrap();
    let tt3 = Template::builder()
        .include_path(ip())
        .pre_process(vec!["config".into(), "header".into()])
        .post_process(vec!["footer".into()])
        .blocks(blocks())
        .error(ErrorSpec::Single("barfed".into()))
        .auto_reset(false)
        .build()
        .unwrap();
    let wrapper_cfg = |wrapper: Vec<String>| {
        Template::builder()
            .include_path(ip())
            .pre_process(vec!["before".into()])
            .post_process(vec!["after".into()])
            .process(vec!["process".into()])
            .wrapper(wrapper)
            .blocks(blocks())
            .error(ErrorSpec::Single("barfed".into()))
            .auto_reset(false)
            .build()
            .unwrap()
    };
    let tt4 = wrapper_cfg(vec!["outer".into()]);
    let tt5 = wrapper_cfg(vec!["outer".into(), "inner".into()]);

    test_expect(
        &tt_test("service"),
        vec![
            ("tt1", tt1),
            ("tt2", tt2),
            ("tt3", tt3),
            ("wrapper", tt4),
            ("nested", tt5),
        ],
        vars(vec![("title", "Joe Random Title".into())]),
    );
}
