mod common;
use common::*;
use template_toolkit::{provider::IncludePathItem, Template};

#[test]
fn process() {
    let base = format!(
        "{}/corpora/Template-Toolkit-3.009/t/test",
        env!("CARGO_MANIFEST_DIR")
    );
    let ip = || {
        vec![
            IncludePathItem::Path(format!("{}/src", base)),
            IncludePathItem::Path(format!("{}/lib", base)),
        ]
    };
    let tt1 = Template::builder()
        .include_path(ip())
        .process(vec!["content".into()])
        .trim(true)
        .build()
        .unwrap();
    let tt2 = Template::builder()
        .include_path(ip())
        .pre_process(vec!["config".into()])
        .process(vec!["header".into(), "content".into()])
        .post_process(vec!["footer".into()])
        .build()
        .unwrap();
    let tt3 = Template::builder()
        .include_path(ip())
        .pre_process(vec!["config".into(), "header.tt2".into()])
        .process(vec![])
        .post_process(vec!["footer".into()])
        .build()
        .unwrap();
    test_expect(
        &tt_test("process"),
        vec![("tt1", tt1), ("tt2", tt2), ("tt3", tt3)],
        vars(vec![("title", "Joe Random Title".into())]),
    );
}
