#![cfg(feature = "dumper")]

mod common;
use common::*;
use template_toolkit::Template;

#[test]
fn dumper() {
    let params = hash(vec![(
        "baz",
        template_toolkit::value::Value::Str("boo".into()),
    )]);
    let tt = Template::builder().build().unwrap();
    test_expect(
        &tt_test("dumper"),
        vec![("default", tt)],
        vars(vec![("params", params)]),
    );
}
