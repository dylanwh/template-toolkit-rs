//! NEXT and LAST outside a loop.  Perl compiles them to a bare `next;` or
//! `last;`, which leaves the innermost bare block of the generated code:
//! a template or BLOCK body, a SWITCH or an anonymous BLOCK.  These cases
//! were checked against Perl TT 3.009.
mod common;
use common::*;
use template_toolkit::Template;

#[test]
fn next_last() {
    test_expect(
        include_str!("corpus/next_last.tt"),
        vec![("default", Template::new())],
        vars(vec![]),
    );
}
