mod common;
use common::*;
use template_toolkit::{constants::CHOMP_ALL, Template};

#[test]
fn format() {
    let params = vars(vec![
        ("a", "alpha".into()),
        ("b", "bravo".into()),
        ("c", "charlie".into()),
        ("d", "delta".into()),
    ]);
    let tt = Template::builder()
        .interpolate(true)
        .post_chomp(CHOMP_ALL)
        .build()
        .unwrap();
    test_expect(
        include_str!("corpus/format.tt"),
        vec![("default", tt)],
        params,
    );
}
