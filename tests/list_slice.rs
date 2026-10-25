//! list.slice as Perl's `[ @$list[$from..$to] ]`: indices past either end
//! give undefined items.  Checked against Perl TT 3.009.
mod common;
use common::*;
use template_toolkit::Template;

#[test]
fn list_slice() {
    test_expect(
        include_str!("corpus/list_slice.tt"),
        vec![("default", Template::new())],
        vars(vec![]),
    );
}
