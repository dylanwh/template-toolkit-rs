mod common;
use common::*;
use template_toolkit::{
    error::ControlFlow,
    value::{CallContext, MethodResult, TemplateObject, Value},
    Template,
};

/// Perl `My::Object`: name() and extra() accessors.
struct MyObject {
    name: String,
    extra: Value,
}
impl TemplateObject for MyObject {
    fn type_name(&self) -> &str {
        "My::Object"
    }
    fn call_method(&self, m: &str, _a: &[Value], _c: CallContext) -> MethodResult {
        match m {
            "name" => MethodResult::Ok(Value::Str(self.name.clone())),
            "extra" => MethodResult::Ok(self.extra.clone()),
            _ => MethodResult::NotFound,
        }
    }
}
fn obj(name: &str, extra: Option<&str>) -> Value {
    Value::object(MyObject {
        name: name.into(),
        extra: extra.map(|e| e.into()).unwrap_or(Value::Undefined),
    })
}

fn sum(l: &Value, _a: &[Value]) -> Result<Value, ControlFlow> {
    let Value::List(l) = l else {
        return Ok(Value::Int(0));
    };
    let n: f64 = l.borrow().iter().map(|v| v.to_number()).sum();
    Ok(if n.fract() == 0.0 {
        Value::Int(n as i64)
    } else {
        Value::Float(n)
    })
}
fn odd(l: &Value, _a: &[Value]) -> Result<Value, ControlFlow> {
    let Value::List(l) = l else {
        return Ok(Value::Undefined);
    };
    Ok(Value::list(
        l.borrow()
            .iter()
            .filter(|v| v.to_int() % 2 != 0)
            .cloned()
            .collect(),
    ))
}
fn jumble(l: &Value, a: &[Value]) -> Result<Value, ControlFlow> {
    let Value::List(rc) = l else {
        return Ok(Value::Undefined);
    };
    let chop = a
        .first()
        .filter(|v| v.is_defined())
        .map(|v| v.to_int())
        .unwrap_or(1)
        .max(0) as usize;
    {
        let mut v = rc.borrow_mut();
        if v.len() > 3 {
            let n = chop.min(v.len());
            let head: Vec<Value> = v.drain(0..n).collect();
            v.extend(head);
        }
    }
    Ok(l.clone())
}

#[test]
fn vmethods_list() {
    let params = vars(vec![
        (
            "metavars",
            strs(&["foo", "bar", "baz", "qux", "wiz", "waz", "woz"]),
        ),
        (
            "people",
            Value::list(vec![
                hash(vec![("id", "tom".into()), ("name", "Tom".into())]),
                hash(vec![("id", "dick".into()), ("name", "Richard".into())]),
                hash(vec![("id", "larry".into()), ("name", "Larry".into())]),
            ]),
        ),
        (
            "primes",
            Value::list(
                [13, 11, 17, 19, 2, 3, 5, 7]
                    .iter()
                    .map(|n| Value::Int(*n))
                    .collect(),
            ),
        ),
        (
            "phones",
            hash(vec![
                ("3141", "Leon".into()),
                ("5131", "Andy".into()),
                ("4131", "Simon".into()),
            ]),
        ),
        (
            "groceries",
            hash(vec![
                ("Flour", 3.into()),
                ("Milk", 1.into()),
                ("Peanut Butter", 21.into()),
            ]),
        ),
        (
            "names",
            Value::list(
                ["Tom", "Dick", "Larry"]
                    .iter()
                    .map(|n| obj(n, None))
                    .collect(),
            ),
        ),
        (
            "more_names",
            Value::list(vec![
                obj("Smith", Some("William")),
                obj("Smith", Some("Andrew")),
                obj("Jones", Some("Peter")),
                obj("Jones", Some("Mark")),
            ]),
        ),
        (
            "numbers",
            Value::list(
                ["1", "02", "10", "12", "021"]
                    .iter()
                    .map(|n| obj(n, None))
                    .collect(),
            ),
        ),
        (
            "duplicates",
            Value::list(
                [1, 1, 2, 2, 3, 3, 4, 4, 5, 5]
                    .iter()
                    .map(|n| Value::Int(*n))
                    .collect(),
            ),
        ),
    ]);
    let tt = Template::new();
    let ctx = tt.context();
    // $Template::Stash::LIST_OPS->{ sum/odd/jumble } plus define_vmethod() calls
    ctx.define_vmethod("list", "sum", sum);
    ctx.define_vmethod("list", "odd", odd);
    ctx.define_vmethod("list", "jumble", jumble);
    ctx.define_vmethod("list", "oddnos", odd);
    ctx.define_vmethod("array", "jumblate", jumble);
    test_expect(
        include_str!("corpus/vmethods_list.tt"),
        vec![("default", tt)],
        params,
    );
}
