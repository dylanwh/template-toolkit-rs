mod common;
use common::*;
use template_toolkit::{constants::CHOMP_ALL, provider::IncludePathItem, Template};

#[test]
fn directive() {
    let tt = Template::builder().build().unwrap();
    let pre = Template::builder().pre_chomp(CHOMP_ALL).build().unwrap();
    let post = Template::builder().post_chomp(CHOMP_ALL).build().unwrap();
    let trim = Template::builder()
        .include_path(vec![IncludePathItem::Path(
            "corpora/Template-Toolkit-3.009/t/test/lib".into(),
        )])
        .trim(true)
        .build()
        .unwrap();
    test_expect(
        &tt_test("directive"),
        vec![("tt", tt), ("pre", pre), ("post", post), ("trim", trim)],
        callsign(),
    );
}
