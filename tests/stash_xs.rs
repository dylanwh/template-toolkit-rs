mod common;
mod stash_support;
use std::{collections::HashMap, rc::Rc};

use common::*;
use stash_support::*;
use template_toolkit::{
    context::Context,
    error::{ControlFlow, TemplateException},
    plugins::PluginFactory,
    stash::Stash,
    value::Value,
    Template,
};

/// The template only does `USE Dumper` for its side effect; the Dumper plugin
/// itself is out of scope, so a stub is registered.
struct DumperStub;
impl PluginFactory for DumperStub {
    fn load(&self, _c: &Context, _a: &[Value]) -> Result<Value, TemplateException> {
        Ok(Value::new_hash())
    }
}

// There is a single Stash implementation in Rust; this runs the stash-xs.t
// suite (otherwise skipped without Template::Stash::XS) against it.
#[test]
fn stash_xs() {
    let mut data = base_data();
    data.insert(
        "correct".into(),
        Value::function(|args, _| {
            Err(ControlFlow::Exception(TemplateException::new(
                "undef",
                args.first().cloned().unwrap_or(Value::Undefined),
            )))
        }),
    );
    data.insert("buggy".into(), Value::object(Buggy));
    // RT#47929: the Perl eval { use No::Such::Module } must not clobber the result
    data.insert(
        "str_eval_die".into(),
        Value::function(|_, _| Ok(Value::Str("str_eval_die returned".into()))),
    );

    let stash = Stash::new(Some(data.clone()));
    check_stash(&stash);
    let r = stash
        .dotop(
            &hash(vec![("foo", Value::Int(10))]),
            &Value::Str("foo".into()),
            &[],
            false,
        )
        .unwrap();
    assert_eq!(r.to_tt_string(), "10");

    let stash_dbg = Stash::builder().params(data.clone()).debug(true).build();
    let plugins = || {
        let mut p: HashMap<String, Rc<dyn PluginFactory>> = HashMap::new();
        p.insert("Dumper".into(), Rc::new(DumperStub));
        p
    };
    let ttlist = vec![
        (
            "default",
            Template::builder()
                .stash(stash)
                .plugins(plugins())
                .build()
                .unwrap(),
        ),
        (
            "warn",
            Template::builder()
                .stash(stash_dbg)
                .plugins(plugins())
                .build()
                .unwrap(),
        ),
    ];
    test_expect(&tt_test("stash-xs"), ttlist, data);
}
