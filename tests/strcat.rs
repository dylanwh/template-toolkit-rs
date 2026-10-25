mod common;
use common::*;
use template_toolkit::Template;

#[test]
fn strcat() {
    test_expect(
        &tt_test("strcat"),
        vec![("default", Template::new())],
        vars(vec![]),
    );
}
