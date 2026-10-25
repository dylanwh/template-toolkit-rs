mod common;
use common::*;
use template_toolkit::{constants::CHOMP_ALL, Template};

#[test]
fn capture() {
    let tt = Template::builder().post_chomp(CHOMP_ALL).build().unwrap();
    let v = vars(vec![("a", "alpha".into()), ("b", "bravo".into())]);
    test_expect(&tt_test("capture"), vec![("default", tt)], v);
}
