mod common;
use std::{collections::HashMap, rc::Rc};

use common::*;
use template_toolkit::{
    context::Context,
    error::TemplateException,
    plugins::PluginFactory,
    value::{CallContext, MethodResult, TemplateObject, Value},
    Template,
};

/// Rust stand-in for a Template::Plugin::Procedural subclass: methods are
/// plain subs looked up by name (with inheritance flattened into the table).
struct Proc(Vec<(&'static str, &'static str)>);

impl TemplateObject for Proc {
    fn type_name(&self) -> &str {
        "Template::Plugin::Procedural"
    }
    fn call_method(&self, method: &str, _args: &[Value], _ctx: CallContext) -> MethodResult {
        match self.0.iter().find(|(k, _)| *k == method) {
            Some((_, v)) => MethodResult::Ok(Value::Str(v.to_string())),
            None => MethodResult::NotFound,
        }
    }
}

struct ProcFactory(Vec<(&'static str, &'static str)>);

impl PluginFactory for ProcFactory {
    fn load(&self, _c: &Context, _a: &[Value]) -> Result<Value, TemplateException> {
        Ok(Value::object(Proc(self.0.clone())))
    }
}

#[test]
fn proc() {
    // Template::Plugin::ProcFoo / ProcBar from t/lib (ProcBar ISA ProcFoo)
    let mut plugins: HashMap<String, Rc<dyn PluginFactory>> = HashMap::new();
    plugins.insert(
        "ProcFoo".into(),
        Rc::new(ProcFactory(vec![
            ("foo", "This is procfoofoo"),
            ("bar", "This is procfoobar"),
        ])),
    );
    plugins.insert(
        "ProcBar".into(),
        Rc::new(ProcFactory(vec![
            ("foo", "This is procfoofoo"),
            ("bar", "This is procbarbar"),
            ("baz", "This is procbarbaz"),
        ])),
    );
    let tt = Template::builder().plugins(plugins).build().unwrap();
    test_expect(
        include_str!("corpus/proc.tt"),
        vec![("default", tt)],
        callsign(),
    );
}
