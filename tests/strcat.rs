mod common;
use common::*;
use template_toolkit::Template;

#[test]
fn strcat() {
    test_expect(
        include_str!("corpus/strcat.tt"),
        vec![("default", Template::new())],
        vars(vec![]),
    );
}
