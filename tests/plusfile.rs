mod common;
use common::*;
use template_toolkit::{provider::IncludePathItem, Template};

#[test]
fn plusfile() {
    let dir = format!(
        "{}/Template-Toolkit-3.009/t/test/src",
        env!("CARGO_MANIFEST_DIR")
    );
    let tt = Template::builder()
        .include_path(vec![IncludePathItem::Path(dir)])
        .build()
        .unwrap();
    test_expect(
        include_str!("corpus/plusfile.tt"),
        vec![("default", tt)],
        vars(vec![]),
    );
}
