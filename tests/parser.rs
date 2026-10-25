mod common;
use common::*;
use template_toolkit::{
    parser::{Parser, StyleConfig},
    value::Value,
    Template,
};

#[test]
fn parser() {
    let mut p2 = Parser::builder()
        .start_tag(r"\[\*".to_string())
        .end_tag(r"\*\]".to_string())
        .anycase(true)
        .pre_chomp(1)
        .v1dollar(true)
        .build()
        .unwrap();

    let s1 = p2
        .new_style(StyleConfig {
            tag_style: Some("metatext".into()),
            pre_chomp: Some(0),
            post_chomp: Some(1),
            ..Default::default()
        })
        .expect("new_style")
        .clone();
    assert_eq!(s1.start_tag, "%%");
    assert_eq!(s1.pre_chomp, 0);
    assert_eq!(s1.post_chomp, 1);

    let s2 = p2.old_style().expect("old_style").clone();
    assert_eq!(s2.start_tag, r"\[\*");
    assert_eq!(s2.pre_chomp, 1);
    assert_eq!(s2.post_chomp, 0);

    let p3 = Parser::builder()
        .tag_style("html".to_string())
        .post_chomp(1)
        .anycase(true)
        .interpolate(true)
        .build()
        .unwrap();
    let p4 = Parser::builder().anycase(false).build().unwrap();

    let tt = vec![
        ("tt1", Template::builder().anycase(true).build().unwrap()),
        ("tt2", Template::builder().parser(p2).build().unwrap()),
        ("tt3", Template::builder().parser(p3).build().unwrap()),
        ("tt4", Template::builder().parser(p4).build().unwrap()),
    ];

    let mut replace = callsign();
    replace.insert(
        "alist".into(),
        Value::list(vec![
            "foo".into(),
            Value::Int(0),
            "bar".into(),
            Value::Int(0),
        ]),
    );
    replace.insert("wintxt".into(), "foo\r\n\r\nbar\r\n\r\nbaz".into());
    replace.insert(
        "data".into(),
        hash(vec![("first", Value::Int(11)), ("last", Value::Int(42))]),
    );
    test_expect(include_str!("corpus/parser.tt"), tt, replace);
}
