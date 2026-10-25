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
fn wrap() {
    test_expect(
        include_str!("corpus/wrap.tt"),
        vec![("default", Template::new())],
        vars(vec![]),
    );
}
