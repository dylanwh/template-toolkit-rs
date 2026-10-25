mod common;
use common::*;
use template_toolkit::{
    value::{CallContext, MethodResult, TemplateObject, Value},
    Template,
};

struct TestObject;
impl TemplateObject for TestObject {
    fn type_name(&self) -> &str {
        "Template::Test::Object"
    }
    fn call_method(&self, method: &str, _args: &[Value], _ctx: CallContext) -> MethodResult {
        match method {
            "nil" => MethodResult::Ok(Value::Undefined),
            _ => MethodResult::NotFound,
        }
    }
}

#[test]
fn assert() {
    let v = vars(vec![
        ("object", Value::object(TestObject)),
        (
            "hash",
            hash(vec![("foo", 10.into()), ("bar", Value::Undefined)]),
        ),
        ("list", Value::list(vec![Value::Undefined])),
        ("subref", Value::function(|_, _| Ok(Value::Undefined))),
        ("nothing", Value::Undefined),
    ]);
    test_expect(&tt_test("assert"), vec![("default", Template::new())], v);
}
