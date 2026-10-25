mod common;
use common::*;
use template_toolkit::Template;

// EVAL_PERL is out of scope (no Perl evaluation); the skeleton corpus is run without it.
#[test]
fn skel() {
    let tt = Template::builder()
        .post_chomp(template_toolkit::constants::CHOMP_ALL)
        .build()
        .unwrap();
    test_expect(
        include_str!("corpus/skel.tt"),
        vec![("default", tt)],
        vars(vec![("a", "alpha".into()), ("b", "bravo".into())]),
    );
}
