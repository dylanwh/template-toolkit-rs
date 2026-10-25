mod common;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use common::*;
use template_toolkit::{
    provider::{IncludePathItem, Provider},
    Template,
};

fn prov(dir: &str) -> Rc<RefCell<Provider>> {
    let d = format!(
        "{}/Template-Toolkit-3.009/t/test/{}",
        env!("CARGO_MANIFEST_DIR"),
        dir
    );
    Rc::new(RefCell::new(
        Provider::builder()
            .include_path(vec![IncludePathItem::Path(d)])
            .build()
            .unwrap(),
    ))
}

#[test]
fn prefix() {
    let src = prov("src");
    let lib = prov("lib");
    let mut pm = HashMap::new();
    pm.insert("src".to_string(), vec![src.clone()]);
    pm.insert("lib".to_string(), vec![lib.clone()]);
    pm.insert("all".to_string(), vec![src.clone(), lib.clone()]);
    let tt = Template::builder()
        .load_templates(vec![src, lib])
        .prefix_map(pm)
        .build()
        .unwrap();
    test_expect(
        include_str!("corpus/prefix.tt"),
        vec![("default", tt)],
        vars(vec![]),
    );
}
