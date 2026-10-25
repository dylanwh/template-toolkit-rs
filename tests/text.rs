mod common;
use common::*;
use template_toolkit::{
    value::{CallContext, MethodResult, TemplateObject, Value},
    Template,
};

/// Perl `Stringy` class with overloaded `""`.
struct Stringy(String);
impl TemplateObject for Stringy {
    fn type_name(&self) -> &str {
        "Stringy"
    }
    fn to_tt_string(&self) -> String {
        self.0.clone()
    }
    fn call_method(&self, _m: &str, _a: &[Value], _c: CallContext) -> MethodResult {
        MethodResult::NotFound
    }
}

#[test]
fn text() {
    let mut v = callsign();
    v.insert(
        "ref".into(),
        Value::function(|a, _| {
            let a = a.first().cloned().unwrap_or(Value::Undefined);
            Ok(Value::Str(format!(
                "{}[{}]",
                a.to_tt_string(),
                a.ref_type()
            )))
        }),
    );
    v.insert("sfoo".into(), Value::object(Stringy("foo".into())));
    v.insert("sbar".into(), Value::object(Stringy("bar".into())));
    let procs = vec![
        ("basic", Template::new()),
        (
            "interp",
            Template::builder().interpolate(true).build().unwrap(),
        ),
    ];
    test_expect(&tt_test("text"), procs, v);
}
