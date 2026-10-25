//! Shared fixtures for stash.t, stash-xs.t and stashc.t.
#![allow(dead_code)]
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

use template_toolkit::{
    error::{ControlFlow, TemplateException},
    stash::Stash,
    value::{CallContext, MethodResult, TemplateObject, Value},
};

pub fn cant_locate(method: &str, package: &str) -> MethodResult {
    MethodResult::Err(ControlFlow::Exception(TemplateException::new(
        "undef",
        format!(
            "Can't locate object method \"{}\" via package \"{}\" at x line 1.\n",
            method, package
        ),
    )))
}

/// package HashObject: a blessed hash { planet => 'World' } with methods.
pub struct HashObject {
    pub hash: Rc<RefCell<HashMap<String, Value>>>,
}

impl HashObject {
    pub fn new() -> Self {
        let mut m = HashMap::new();
        m.insert("planet".to_string(), Value::Str("World".into()));
        HashObject {
            hash: Rc::new(RefCell::new(m)),
        }
    }
}

impl TemplateObject for HashObject {
    fn type_name(&self) -> &str {
        "HashObject"
    }
    fn call_method(&self, method: &str, _a: &[Value], _c: CallContext) -> MethodResult {
        match method {
            "hello" => {
                let p = self
                    .hash
                    .borrow()
                    .get("planet")
                    .map(|v| v.to_tt_string())
                    .unwrap_or_default();
                MethodResult::Ok(Value::Str(format!("Hello {}", p)))
            }
            "goodbye" => cant_locate("no_such_method", "HashObject"),
            "now_is_the_time_to_test_a_very_long_method_to_see_what_happens" => {
                cant_locate("this_method_does_not_exist", "HashObject")
            }
            _ => MethodResult::NotFound,
        }
    }
    fn as_hash(&self) -> Option<Rc<RefCell<HashMap<String, Value>>>> {
        Some(self.hash.clone())
    }
}

/// package CmpOverloadObject: comparison overloads die, so they must never be used.
pub struct CmpOverloadObject;
impl TemplateObject for CmpOverloadObject {
    fn type_name(&self) -> &str {
        "CmpOverloadObject"
    }
    fn call_method(&self, method: &str, _a: &[Value], _c: CallContext) -> MethodResult {
        match method {
            "hello" => MethodResult::Ok(Value::Str("Hello".into())),
            _ => MethodResult::NotFound,
        }
    }
}

/// package Numbersome
pub struct Numbersome(pub String);
impl TemplateObject for Numbersome {
    fn type_name(&self) -> &str {
        "Numbersome"
    }
    fn to_tt_string(&self) -> String {
        format!("PASS: stringified {}", self.0)
    }
    fn call_method(&self, method: &str, _a: &[Value], _c: CallContext) -> MethodResult {
        match method {
            "things" => {
                MethodResult::Ok(Value::list(vec!["foo".into(), "bar".into(), "baz".into()]))
            }
            _ => MethodResult::NotFound,
        }
    }
}

pub struct GetNumbersome;
impl TemplateObject for GetNumbersome {
    fn type_name(&self) -> &str {
        "GetNumbersome"
    }
    fn call_method(&self, method: &str, _a: &[Value], _c: CallContext) -> MethodResult {
        match method {
            "num" => MethodResult::Ok(Value::object(Numbersome("from GetNumbersome".into()))),
            _ => MethodResult::NotFound,
        }
    }
}

/// package Buggy: croak dies with its arguments (a list ref) as the error.
pub struct Buggy;
impl TemplateObject for Buggy {
    fn type_name(&self) -> &str {
        "Buggy"
    }
    fn call_method(&self, method: &str, args: &[Value], _c: CallContext) -> MethodResult {
        match method {
            "croak" => MethodResult::Err(ControlFlow::Exception(TemplateException::new(
                "undef",
                args.first().cloned().unwrap_or(Value::Undefined),
            ))),
            _ => MethodResult::NotFound,
        }
    }
}

/// A hash blessed into 'AnObject' (no methods): hash/list vmethod fallbacks apply.
pub struct AnObject(pub Rc<RefCell<HashMap<String, Value>>>);
impl AnObject {
    pub fn value() -> Value {
        let mut m = HashMap::new();
        m.insert("name".to_string(), Value::Str("an object".into()));
        Value::object(AnObject(Rc::new(RefCell::new(m))))
    }
}
impl TemplateObject for AnObject {
    fn type_name(&self) -> &str {
        "AnObject"
    }
    fn call_method(&self, _m: &str, _a: &[Value], _c: CallContext) -> MethodResult {
        MethodResult::NotFound
    }
    fn as_hash(&self) -> Option<Rc<RefCell<HashMap<String, Value>>>> {
        Some(self.0.clone())
    }
}

pub fn h(pairs: Vec<(&str, Value)>) -> Value {
    Value::hash(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

/// The `$data` hash shared by the stash tests (minus per-test extras).
pub fn base_data() -> HashMap<String, Value> {
    let count = Rc::new(Cell::new(20i64));
    let mut d: HashMap<String, Value> = HashMap::new();
    d.insert("foo".into(), Value::Int(10));
    d.insert("bar".into(), h(vec![("baz", Value::Int(20))]));
    d.insert(
        "baz".into(),
        Value::function(move |args, _| {
            count.set(count.get() + 10);
            Ok(h(vec![
                ("boz", Value::Int(count.get())),
                (
                    "biz",
                    match args.first() {
                        Some(v) if v.is_truthy() => v.clone(),
                        _ => Value::Str("<undef>".into()),
                    },
                ),
            ]))
        }),
    );
    d.insert("obj".into(), AnObject::value());
    d.insert("bop".into(), Value::function(|_, _| Ok(AnObject::value())));
    d.insert(
        "listobj".into(),
        Value::list(vec![Value::Int(10), Value::Int(20), Value::Int(30)]),
    );
    d.insert("hashobj".into(), Value::object(HashObject::new()));
    d.insert("cmp_ol".into(), Value::object(CmpOverloadObject));
    d.insert(
        "clean".into(),
        Value::function(|args, _| {
            let e = args.first().map(|v| v.to_tt_string()).unwrap_or_default();
            let re = regex::Regex::new(r"(?s)(\s*\(.*?\))?\s+at.*$").unwrap();
            Ok(Value::Str(re.replace(&e, "").to_string()))
        }),
    );
    d
}

fn expect_str(got: Result<Value, ControlFlow>, want: &str, what: &str) {
    match got {
        Ok(v) => assert_eq!(v.to_tt_string(), want, "{}", what),
        Err(e) => panic!("{}: error {:?}", what, e),
    }
}

/// The direct Stash API checks common to stash.t, stash-xs.t and stashc.t.
pub fn check_stash(stash: &Stash) {
    let g = |s: &str| stash.get(&Value::Str(s.into()), None);
    expect_str(g("foo"), "10", "foo");
    let l = Value::list(vec![
        "bar".into(),
        Value::Int(0),
        "baz".into(),
        Value::Int(0),
    ]);
    expect_str(stash.get(&l, None), "20", "[bar,0,baz,0]");
    expect_str(g("bar.baz"), "20", "bar.baz");
    expect_str(g("bar(10).baz"), "20", "bar(10).baz");
    expect_str(g("baz.boz"), "30", "baz.boz");
    expect_str(g("baz.boz"), "40", "baz.boz again");
    expect_str(g("baz.biz"), "<undef>", "baz.biz");
    expect_str(g("baz(50).biz"), "<undef>", "args are ignored");
    stash
        .set(&Value::Str("bar.buz".into()), Value::Int(100), false)
        .unwrap();
    expect_str(g("bar.buz"), "100", "bar.buz");
}
