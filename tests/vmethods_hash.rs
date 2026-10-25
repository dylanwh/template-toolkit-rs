mod common;
use common::*;
use template_toolkit::{error::ControlFlow, value::Value, Template};

fn dump(h: &Value, _a: &[Value]) -> Result<Value, ControlFlow> {
    let Value::Hash(h) = h else {
        return Ok(Value::Undefined);
    };
    let h = h.borrow();
    let mut keys: Vec<&String> = h.keys().collect();
    keys.sort();
    let items: Vec<String> = keys
        .iter()
        .map(|k| format!("{} => '{}'", k, h[*k].to_tt_string()))
        .collect();
    Ok(Value::Str(format!("{{ {} }}", items.join(", "))))
}

#[test]
fn vmethods_hash() {
    let params = vars(vec![
        ("hash", hash(vec![("a", "b".into()), ("c", "d".into())])),
        (
            "uhash",
            hash(vec![("tobe", "2b".into()), ("nottobe", Value::Undefined)]),
        ),
    ]);
    let tt = Template::new();
    tt.context().define_vmethod("hash", "dump", dump);
    test_expect(&tt_test("vmethods/hash"), vec![("default", tt)], params);
}
