mod common;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use common::*;
use template_toolkit::{
    value::{CallContext, MethodResult, TemplateObject, Value},
    Template,
};

/// Perl `Foo`: a blessed hash with `present` and `reverse` methods.
struct Foo(Rc<RefCell<HashMap<String, Value>>>);
impl Foo {
    fn render(&self, rev: bool) -> String {
        let h = self.0.borrow();
        let mut keys: Vec<&String> = h.keys().collect();
        keys.sort();
        if rev {
            keys.reverse();
        }
        let items: Vec<String> = keys
            .iter()
            .map(|k| format!("{} => {}", k, h[*k].to_tt_string()))
            .collect();
        format!("{{ {} }}", items.join(", "))
    }
}
impl TemplateObject for Foo {
    fn type_name(&self) -> &str {
        "Foo"
    }
    fn call_method(&self, m: &str, _a: &[Value], _c: CallContext) -> MethodResult {
        match m {
            "present" => MethodResult::Ok(Value::Str(self.render(false))),
            "reverse" => MethodResult::Ok(Value::Str(self.render(true))),
            _ => MethodResult::NotFound,
        }
    }
    fn as_hash(&self) -> Option<Rc<RefCell<HashMap<String, Value>>>> {
        Some(self.0.clone())
    }
}

/// Perl `Blessed::List`: a blessed array with an `as_list` method.
struct BlessedList(Rc<RefCell<Vec<Value>>>);
impl TemplateObject for BlessedList {
    fn type_name(&self) -> &str {
        "Blessed::List"
    }
    fn call_method(&self, m: &str, _a: &[Value], _c: CallContext) -> MethodResult {
        match m {
            "as_list" => MethodResult::Ok(Value::List(self.0.clone())),
            _ => MethodResult::NotFound,
        }
    }
    fn as_list(&self) -> Option<Rc<RefCell<Vec<Value>>>> {
        Some(self.0.clone())
    }
}

#[test]
fn view() {
    let foo: HashMap<String, Value> = [
        ("pi".to_string(), Value::Str("3.14".into())),
        ("e".to_string(), Value::Str("2.718".into())),
    ]
    .into();
    let params = vars(vec![
        ("foo", Value::object(Foo(Rc::new(RefCell::new(foo))))),
        (
            "blessed_list",
            Value::object(BlessedList(Rc::new(RefCell::new(vec![
                "Hello".into(),
                "World".into(),
            ])))),
        ),
    ]);

    let template = Template::new();
    let context = template.context();
    let _view = context.view(HashMap::new()).expect("view");
    let view = context
        .view(vars(vec![("prefix", "my".into())]))
        .expect("view with prefix");
    assert_eq!(view.inner.borrow().prefix, "my");

    let cfg = Template::builder()
        .views(vec![
            (
                "bottom".to_string(),
                vars(vec![("prefix", "bottom/".into())]),
            ),
            (
                "middle".to_string(),
                vars(vec![
                    ("prefix", "middle/".into()),
                    ("base", "bottom".into()),
                ]),
            ),
        ])
        .build()
        .unwrap();
    test_expect(
        include_str!("corpus/view.tt"),
        vec![("default", cfg)],
        params,
    );
}
