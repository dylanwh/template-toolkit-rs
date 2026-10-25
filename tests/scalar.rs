mod common;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use common::*;
use template_toolkit::{
    value::{CallContext, MethodResult, TemplateObject, Value},
    Template,
};

// Template::Test::HashObject / ListObject: `bar` is context-sensitive.
struct Ctx {
    kind: &'static str,
    list: bool,
}
impl TemplateObject for Ctx {
    fn type_name(&self) -> &str {
        "Template::Test::Object"
    }
    fn call_method(&self, method: &str, _a: &[Value], c: CallContext) -> MethodResult {
        match method {
            "bar" => match c {
                CallContext::List => MethodResult::Ok(strs(&[
                    self.kind, "object", "method", "called", "in", "array", "context",
                ])),
                CallContext::Scalar => MethodResult::Ok(Value::Str(format!(
                    "{} object method called in scalar context",
                    self.kind
                ))),
            },
            _ => MethodResult::NotFound,
        }
    }
    fn as_hash(&self) -> Option<Rc<RefCell<HashMap<String, Value>>>> {
        if self.list {
            None
        } else {
            Some(Rc::new(RefCell::new(HashMap::new())))
        }
    }
    fn as_list(&self) -> Option<Rc<RefCell<Vec<Value>>>> {
        if self.list {
            Some(Rc::new(RefCell::new(Vec::new())))
        } else {
            None
        }
    }
}

#[test]
fn scalar() {
    let params = vars(vec![
        (
            "hashobj",
            Value::object(Ctx {
                kind: "hash",
                list: false,
            }),
        ),
        (
            "listobj",
            Value::object(Ctx {
                kind: "list",
                list: true,
            }),
        ),
        (
            "subref",
            Value::function(|args, c| {
                Ok(match c {
                    CallContext::List => {
                        let mut v: Vec<Value> = "subroutine called in array context"
                            .split(' ')
                            .map(|s| Value::Str(s.into()))
                            .collect();
                        v.extend(args.iter().cloned());
                        Value::list(v)
                    }
                    CallContext::Scalar => {
                        let a: Vec<String> = args.iter().map(|v| v.to_tt_string()).collect();
                        Value::Str(format!(
                            "subroutine called in scalar context {}",
                            a.join(" ")
                        ))
                    }
                })
            }),
        ),
    ]);
    test_expect(
        &tt_test("scalar"),
        vec![("default", Template::new())],
        params,
    );
}
