mod common;
use common::*;
use template_toolkit::{value::Value, Template};

#[test]
fn list() {
    let (a, b, c, d, e) = ("alpha", "bravo", "charlie", "delta", "echo");
    let (r, j, s, t, y, f, z) = (
        "romeo", "juliet", "sierra", "tango", "yankee", "foxtrot", "zulu",
    );
    let rec = |id: &str, name: &str, rank: &str| {
        hash(vec![
            ("id", id.into()),
            ("name", name.into()),
            ("rank", rank.into()),
        ])
    };
    let inst = |name: &str, url: &str| hash(vec![("name", name.into()), ("url", url.into())]);
    let params = vars(vec![
        ("a", a.into()),
        ("b", b.into()),
        ("c", c.into()),
        ("d", d.into()),
        ("e", e.into()),
        ("data", strs(&[r, j, s, t, y, e, f, z])),
        (
            "days",
            strs(&["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]),
        ),
        (
            "wxyz",
            Value::list(vec![
                rec("zulu", "Zebedee", "aa"),
                rec("yankee", "Yinyang", "ba"),
                rec("x-ray", "Xeexeez", "ab"),
                rec("whisky", "Warlock", "bb"),
            ]),
        ),
        (
            "inst",
            Value::list(vec![
                inst("piano", "/roses.html"),
                inst("flute", "/blow.html"),
                inst("organ", "/tulips.html"),
            ]),
        ),
        (
            "nest",
            Value::list(vec![
                Value::list(vec![Value::Int(3), Value::Int(1), Value::Int(4)]),
                Value::list(vec![
                    Value::Int(2),
                    Value::list(vec![Value::Int(7), Value::Int(1), Value::Int(8)]),
                ]),
            ]),
        ),
    ]);
    let tt = Template::builder().build().unwrap();
    test_expect(
        include_str!("corpus/list.tt"),
        vec![("default", tt)],
        params,
    );
}
