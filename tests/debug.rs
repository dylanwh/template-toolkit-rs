mod common;
use common::*;
use template_toolkit::{constants::*, provider::IncludePathItem, value::Value, Template};

#[test]
fn debug_flags_api() {
    let flags = debug_flags("dirs, stash").expect("created flags");
    assert_eq!(flags.as_u32(), Some(DEBUG_DIRS | DEBUG_STASH));
    let back = debug_flags(flags.as_u32().unwrap()).expect("got more flags back");
    assert_eq!(back.as_str(), Some("dirs, stash"));
    let err = debug_flags("bad stupid").unwrap_err();
    assert_eq!(err, "unknown debug flag: bad");
}

#[test]
fn debug() {
    let dir = "Template-Toolkit-3.009/t/test";
    let ip = || {
        vec![
            IncludePathItem::Path(format!("{dir}/src")),
            IncludePathItem::Path(format!("{dir}/lib")),
        ]
    };
    let vars = vars(vec![
        ("foo", Value::Int(10)),
        ("bar", Value::Int(20)),
        (
            "baz",
            hash(vec![("ping", Value::Int(100)), ("pong", Value::Int(200))]),
        ),
    ]);
    let fmt = "<!-- $file line $line : [% $text %] -->";
    let tt = Template::builder()
        .debug(DebugFlagsInput::Number(0))
        .include_path(ip())
        .debug_format(fmt.to_string())
        .build()
        .unwrap();
    let tt2 = Template::builder()
        .debug(DebugFlagsInput::Number(DEBUG_DIRS))
        .include_path(ip())
        .build()
        .unwrap();
    let ttd = Template::builder()
        .debug("dirs, vars".into())
        .include_path(ip())
        .debug_format(fmt.to_string())
        .build()
        .unwrap();
    test_expect(
        include_str!("corpus/debug.tt"),
        vec![("default", tt), ("debug", ttd), ("debug2", tt2)],
        vars,
    );
}
