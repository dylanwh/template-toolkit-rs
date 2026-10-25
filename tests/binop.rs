mod common;
use std::{cell::Cell, rc::Rc};

use common::*;
use template_toolkit::{constants::CHOMP_ALL, value::Value, Template};

#[test]
fn binop() {
    let counter = Rc::new(Cell::new(0i64));
    let (c1, c2, c3, c4) = (
        counter.clone(),
        counter.clone(),
        counter.clone(),
        counter.clone(),
    );
    let params = vars(vec![
        ("yes", 1.into()),
        ("no", 0.into()),
        ("true", "this is true".into()),
        ("false", "0".into()),
        ("happy", "yes".into()),
        ("sad", "".into()),
        ("ten", 10.into()),
        ("twenty", 20.into()),
        (
            "alpha",
            Value::function(move |_, _| {
                c1.set(c1.get() + 1);
                Ok(Value::Int(c1.get()))
            }),
        ),
        (
            "omega",
            Value::function(move |_, _| {
                c2.set(c2.get() + 10);
                Ok(Value::Int(0))
            }),
        ),
        (
            "count",
            Value::function(move |_, _| Ok(Value::Int(c3.get()))),
        ),
        (
            "reset",
            Value::function(move |_, _| Ok(Value::from(c4.get() == 0))),
        ),
    ]);
    let tt = Template::builder()
        .interpolate(true)
        .post_chomp(CHOMP_ALL)
        .build()
        .unwrap();
    test_expect(&tt_test("binop"), vec![("default", tt)], params);
}
