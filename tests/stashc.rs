mod common;
mod stash_support;
use common::*;
use stash_support::*;
use template_toolkit::{
    stash::Stash,
    value::{CallContext, Value},
    Template,
};

#[test]
fn stashc() {
    let mut data = base_data();
    data.insert(
        "numbers".into(),
        Value::function(|_, c| {
            Ok(match c {
                CallContext::List => Value::list(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
                CallContext::Scalar => Value::Str("one two three".into()),
            })
        }),
    );
    let stash = Stash::new_context(Some(data));
    check_stash(&stash);
    let tt = Template::builder().stash(stash).build().unwrap();
    test_expect(
        &tt_test("stashc"),
        vec![("default", tt)],
        Default::default(),
    );
}
