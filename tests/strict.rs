mod common;
use common::*;
use template_toolkit::{value::Value, Template};

#[test]
fn strict() {
    let tt = Template::builder().strict(true).build().unwrap();
    test_expect(
        &tt_test("strict"),
        vec![("default", tt)],
        vars(vec![
            ("foo", Value::Int(10)),
            ("bar", Value::Undefined),
            ("baz", hash(vec![("boz", Value::Undefined)])),
        ]),
    );
}
