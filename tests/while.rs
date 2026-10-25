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
fn while_() {
    use std::{cell::RefCell, rc::Rc};
    template_toolkit::context::set_while_max(100);
    let list = ["x-ray", "yankee", "zulu"];
    let pending: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(vec![]));
    let (p1, p2) = (pending.clone(), pending.clone());
    let params = vars(vec![
        ("a", "alpha".into()),
        ("b", "bravo".into()),
        ("c", "charlie".into()),
        ("d", "delta".into()),
        (
            "dec",
            Value::function(|a, _| Ok(Value::Int(a.first().map(|v| v.to_int()).unwrap_or(0) - 1))),
        ),
        (
            "inc",
            Value::function(|a, _| Ok(Value::Int(a.first().map(|v| v.to_int()).unwrap_or(0) + 1))),
        ),
        (
            "reset",
            Value::function(move |_, _| {
                *p1.borrow_mut() = list.iter().map(|s| s.to_string()).collect();
                Ok(Value::Str("Reset list\n".into()))
            }),
        ),
        (
            "next",
            Value::function(move |_, _| {
                let mut p = p2.borrow_mut();
                Ok(if p.is_empty() {
                    Value::Undefined
                } else {
                    Value::Str(p.remove(0))
                })
            }),
        ),
        ("true", Value::Int(1)),
    ]);
    let tt = Template::builder()
        .interpolate(true)
        .post_chomp(CHOMP_ONE)
        .build()
        .unwrap();
    test_expect(&tt_test("while"), vec![("default", tt)], params);
}
