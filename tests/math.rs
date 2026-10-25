mod common;
use common::*;
use template_toolkit::Template;

#[test]
fn math() {
    let tt = Template::builder().build().unwrap();
    test_expect(
        include_str!("corpus/math.tt"),
        vec![("default", tt)],
        vars(vec![]),
    );
}

#[test]
fn math_rand() {
    // Upstream also asserts that no Perl warnings are emitted; only successful
    // processing applies to the Rust port.
    let t = Template::builder().build().unwrap();
    let out = t
        .process_text(
            "[% USE Math -%]\nrand  with arg:    [% Math.rand(1000000) %]\nrand  without arg: [% Math.rand %]\nsrand with arg:    [% Math.srand(1000000) %]\nsrand without arg: [% Math.srand %]\n",
            None,
        )
        .expect("process");
    assert!(out.contains("rand  with arg:"));
}
