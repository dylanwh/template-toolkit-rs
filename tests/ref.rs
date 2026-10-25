mod common;
use common::*;
use template_toolkit::{
    value::{CallContext, Value},
    Template,
};

fn joined(args: &[Value]) -> String {
    args.iter()
        .map(|v| v.to_tt_string())
        .collect::<Vec<_>>()
        .join(", ")
}

#[test]
fn ref_() {
    let params = vars(vec![
        (
            "a",
            Value::function(|args, _| Ok(Value::Str(format!("a sub [{}]", joined(args))))),
        ),
        (
            "j",
            hash(vec![
                ("k", Value::Int(3)),
                ("l", Value::Int(5)),
                (
                    "m",
                    hash(vec![(
                        "n",
                        Value::function(|args, _| {
                            Ok(Value::Str(format!("nsub [{}]", joined(args))))
                        }),
                    )]),
                ),
            ]),
        ),
        (
            "z",
            Value::function(|args, ctx: CallContext| match args.first() {
                Some(Value::Function(f)) => {
                    let r = f.call(&[Value::Int(10), Value::Int(20), Value::Int(30)], ctx)?;
                    Ok(Value::Str(format!("z called {}", r.to_tt_string())))
                }
                _ => Ok(Value::Str("z called ".into())),
            }),
        ),
    ]);
    test_expect(&tt_test("ref"), vec![("default", Template::new())], params);
}
