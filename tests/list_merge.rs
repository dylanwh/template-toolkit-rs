//! list.merge with arguments that aren't lists, which Perl's
//! `map ref eq 'ARRAY' ? @$_ : undef` drops.  Checked against Perl TT 3.009.
mod common;
use common::*;
use template_toolkit::Template;

#[test]
fn list_merge() {
    test_expect(
        include_str!("corpus/list_merge.tt"),
        vec![("default", Template::new())],
        vars(vec![]),
    );
}
