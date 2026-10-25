mod common;
use common::*;
use template_toolkit::{
    value::{CallContext, MethodResult, TemplateObject, Value},
    Template,
};

fn callsign() -> Vars {
    let w = [
        "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel", "india",
        "juliet", "kilo", "lima", "mike", "november", "oscar", "papa", "quebec", "romeo", "sierra",
        "tango", "umbrella", "victor", "whisky", "x-ray", "yankee", "zulu",
    ];
    ('a'..='z')
        .zip(w)
        .map(|(k, v)| (k.to_string(), Value::from(v)))
        .collect()
}

// Port of MyObj::args: trailing hash is the named-args hash.
fn args_str(args: &[Value]) -> String {
    let mut pos: Vec<Value> = args.to_vec();
    let mut named: Vec<(String, String)> = Vec::new();
    if let Some(Value::Hash(h)) = pos.last().cloned() {
        pos.pop();
        for (k, v) in h.borrow().iter() {
            named.push((k.clone(), v.to_tt_string()));
        }
    }
    named.sort();
    format!(
        "  ARGS: [ {} ]\n NAMED: {{ {} }}\n",
        pos.iter()
            .map(|v| v.to_tt_string())
            .collect::<Vec<_>>()
            .join(", "),
        named
            .iter()
            .map(|(k, v)| format!("{} => {}", k, v))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

struct MyObj;
impl TemplateObject for MyObj {
    fn type_name(&self) -> &str {
        "MyObj"
    }
    fn call_method(&self, method: &str, args: &[Value], _ctx: CallContext) -> MethodResult {
        match method {
            "foo" => MethodResult::Ok(Value::Str(format!("object:\n{}", args_str(args)))),
            _ => MethodResult::NotFound,
        }
    }
}

#[test]
fn args() {
    let mut v = callsign();
    v.insert(
        "args".into(),
        Value::function(|a, _| Ok(Value::Str(args_str(a)))),
    );
    v.insert("obj".into(), Value::object(MyObj));
    let tt = Template::builder().interpolate(true).build().unwrap();
    test_expect(&tt_test("args"), vec![("default", tt)], v);
}
