mod common;
use common::*;
use template_toolkit::{constants::CHOMP_ALL, Template};

#[test]
fn datafile() {
    let tt = Template::builder()
        .interpolate(true)
        .post_chomp(CHOMP_ALL)
        .build()
        .unwrap();
    let base = "Template-Toolkit-3.009/t/test/lib";
    let v = vars(vec![(
        "datafile",
        strs(&[&format!("{base}/udata1"), &format!("{base}/udata2")]),
    )]);
    test_expect(include_str!("corpus/datafile.tt"), vec![("default", tt)], v);
}
