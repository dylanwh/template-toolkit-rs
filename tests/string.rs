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
fn string() {
    test_expect(
        &tt_test("string"),
        vec![("default", Template::new())],
        vars(vec![]),
    );
}
