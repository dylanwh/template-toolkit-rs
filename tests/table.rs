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
fn table() {
    let alphabet: Vec<String> = ('a'..='z').map(|c| c.to_string()).collect();
    let refs: Vec<&str> = alphabet.iter().map(|s| s.as_str()).collect();
    let params = vars(vec![("alphabet", strs(&refs)), ("empty", strs(&[]))]);
    let tt = Template::builder().post_chomp(CHOMP_ONE).build().unwrap();
    test_expect(
        include_str!("corpus/table.tt"),
        vec![("default", tt)],
        params,
    );
}
