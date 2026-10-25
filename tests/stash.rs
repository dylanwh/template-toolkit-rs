mod common;
mod stash_support;
use common::*;
use stash_support::*;
use template_toolkit::{
    constants::{DebugFlagsInput, DEBUG_UNDEF},
    stash::Stash,
    value::Value,
    Template,
};

#[test]
fn stash() {
    let mut data = base_data();
    data.insert("num".into(), Value::object(Numbersome("Numbersome".into())));
    data.insert("getnum".into(), Value::object(GetNumbersome));

    let stash = Stash::new(Some(data.clone()));
    check_stash(&stash);
    let r = stash
        .dotop(
            &hash(vec![("foo", Value::Int(10))]),
            &Value::Str("foo".into()),
            &[],
            false,
        )
        .unwrap();
    assert_eq!(r.to_tt_string(), "10");

    let ttlist = vec![
        ("default", Template::new()),
        (
            "warn",
            Template::builder()
                .debug(DebugFlagsInput::Number(DEBUG_UNDEF))
                .debug_format(String::new())
                .build()
                .unwrap(),
        ),
    ];
    test_expect(include_str!("corpus/stash.tt"), ttlist, data);
}
