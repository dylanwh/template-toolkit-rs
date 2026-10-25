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
fn switch() {
    let tt = Template::builder().post_chomp(CHOMP_ONE).build().unwrap();
    test_expect(&tt_test("switch"), vec![("default", tt)], callsign());
}
