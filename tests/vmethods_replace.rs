mod common;
use common::*;
#[allow(unused_imports)]
use template_toolkit::constants::*;
#[allow(unused_imports)]
use template_toolkit::provider::IncludePathItem;
#[allow(unused_imports)]
use template_toolkit::value::Value;
use template_toolkit::Template;

#[test]
fn vmethods_replace() {
    test_expect(
        &tt_test("vmethods/replace"),
        vec![("default", Template::new())],
        vars(vec![]),
    );
}
