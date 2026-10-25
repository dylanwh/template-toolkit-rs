mod common;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use common::*;
use template_toolkit::{
    error::{ControlFlow, TemplateException},
    value::{CallContext, MethodResult, TemplateObject, Value},
    Template,
};

// package T1: a hash-based object whose `die` method dies.
struct T1 {
    hash: Rc<RefCell<HashMap<String, Value>>>,
}

impl TemplateObject for T1 {
    fn type_name(&self) -> &str {
        "T1"
    }
    fn call_method(&self, method: &str, _a: &[Value], _c: CallContext) -> MethodResult {
        match method {
            "die" => MethodResult::Err(ControlFlow::Exception(TemplateException::new(
                "undef",
                "barfed up\n",
            ))),
            _ => MethodResult::NotFound, // "Can't locate object method": falls back to hash key
        }
    }
    fn as_hash(&self) -> Option<Rc<RefCell<HashMap<String, Value>>>> {
        Some(self.hash.clone())
    }
}

// package TestObject: methods plus an AUTOLOAD that reads/writes PARAMS.
struct TestObject {
    params: RefCell<HashMap<String, Value>>,
    day: RefCell<i64>,
}

const DAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

impl TemplateObject for TestObject {
    fn type_name(&self) -> &str {
        "TestObject"
    }
    fn call_method(&self, method: &str, args: &[Value], _c: CallContext) -> MethodResult {
        let s = |t: &str| MethodResult::Ok(Value::Str(t.to_string()));
        match method {
            "yesterday" => s("Love was such an easy game to play..."),
            "today" => s("Live for today and die for tomorrow."),
            "tomorrow" => {
                let n = match args.first() {
                    Some(v) if v.is_defined() => v.to_int(),
                    _ => {
                        let mut d = self.day.borrow_mut();
                        let n = *d;
                        *d += 1;
                        n
                    }
                };
                s(DAYS[n.rem_euclid(7) as usize])
            }
            "belief" => {
                let b = args
                    .iter()
                    .map(|v| v.to_tt_string())
                    .collect::<Vec<_>>()
                    .join(" and ");
                let b = if b.is_empty() {
                    "<nothing>".to_string()
                } else {
                    b
                };
                s(&format!("Oh I believe in {}.", b))
            }
            "concat" => {
                let v = format!(
                    "ARGS: {}",
                    args.iter()
                        .map(|v| v.to_tt_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                self.params
                    .borrow_mut()
                    .insert("args".into(), Value::Str(v.clone()));
                s(&v)
            }
            "_private" => MethodResult::Err(ControlFlow::Exception(TemplateException::new(
                "undef",
                "illegal call to private method _private()\n",
            ))),
            name => {
                // AUTOLOAD
                let cur = self
                    .params
                    .borrow()
                    .get(name)
                    .cloned()
                    .unwrap_or(Value::Undefined);
                if let Value::Function(f) = &cur {
                    return match f.call(args, CallContext::Scalar) {
                        Ok(v) => MethodResult::Ok(v),
                        Err(e) => MethodResult::Err(e),
                    };
                }
                if let Some(v) = args.first() {
                    self.params.borrow_mut().insert(name.to_string(), v.clone());
                    MethodResult::Ok(v.clone())
                } else {
                    MethodResult::Ok(cur)
                }
            }
        }
    }
}

// package Stringy: overloaded stringification.
fn stringy(text: &str) -> Value {
    // stringify => "stringified '$$self'"
    Value::object(StringyObj(text.to_string()))
}

struct StringyObj(String);
impl TemplateObject for StringyObj {
    fn type_name(&self) -> &str {
        "Stringy"
    }
    fn to_tt_string(&self) -> String {
        format!("stringified '{}'", self.0)
    }
    fn call_method(&self, method: &str, _a: &[Value], _c: CallContext) -> MethodResult {
        match method {
            "stringify" => MethodResult::Ok(Value::Str(self.to_tt_string())),
            _ => MethodResult::NotFound,
        }
    }
}

// package Numbersome: '""' => PASS (the numified form would FAIL).
struct Numbersome(String);
impl TemplateObject for Numbersome {
    fn type_name(&self) -> &str {
        "Numbersome"
    }
    fn to_tt_string(&self) -> String {
        format!("PASS: stringified {}", self.0)
    }
    fn call_method(&self, method: &str, _a: &[Value], _c: CallContext) -> MethodResult {
        match method {
            "things" => MethodResult::Ok(strs(&["foo", "bar", "baz"])),
            _ => MethodResult::NotFound,
        }
    }
}

fn numbersome(text: &str) -> Value {
    Value::object(Numbersome(text.to_string()))
}

struct GetNumbersome;
impl TemplateObject for GetNumbersome {
    fn type_name(&self) -> &str {
        "GetNumbersome"
    }
    fn call_method(&self, method: &str, _a: &[Value], _c: CallContext) -> MethodResult {
        match method {
            "num" => MethodResult::Ok(numbersome("from GetNumbersome")),
            _ => MethodResult::NotFound,
        }
    }
}

#[test]
fn object() {
    let objconf: HashMap<String, Value> = [("a", "alpha"), ("b", "bravo"), ("w", "whisky")]
        .iter()
        .map(|(k, v)| (k.to_string(), Value::Str(v.to_string())))
        .collect();
    let mut params = callsign();
    params.extend(vars(vec![
        (
            "thing",
            Value::object(TestObject {
                params: RefCell::new(objconf),
                day: RefCell::new(0),
            }),
        ),
        ("string", stringy("Test String")),
        (
            "t1",
            Value::object(T1 {
                hash: Rc::new(RefCell::new(HashMap::from([(
                    "a".to_string(),
                    Value::Int(10),
                )]))),
            }),
        ),
        ("num", numbersome("Numbersome")),
        ("getnum", Value::object(GetNumbersome)),
    ]));
    let tt = Template::builder().interpolate(true).build().unwrap();
    test_expect(&tt_test("object"), vec![("default", tt)], params);
}
