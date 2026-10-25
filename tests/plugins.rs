mod common;
use std::{collections::HashMap, rc::Rc};

use common::*;
use template_toolkit::{
    context::Context,
    error::TemplateException,
    filters::{FilterDef, FilterFactoryOutcome},
    plugins::PluginFactory,
    value::{CallContext, MethodResult, TemplateObject, Value},
    Template,
};

/// Stand-in for MyPlugs::Foo / Bar / Baz: new() stores VALUE, output() reports it.
struct Mod {
    what: &'static str,
    value: Value,
}

impl TemplateObject for Mod {
    fn type_name(&self) -> &str {
        "MyPlugs"
    }
    fn call_method(&self, method: &str, _a: &[Value], _c: CallContext) -> MethodResult {
        match method {
            "output" => MethodResult::Ok(Value::Str(
                format!(
                    "This is the {} , value is {}",
                    self.what,
                    self.value.to_tt_string()
                )
                .replace(" ,", ","),
            )),
            _ => MethodResult::NotFound,
        }
    }
}

struct ModFactory {
    what: &'static str,
    // Baz is a plain module: new($class, $value) with no context argument
    // (the arguments Template passes are otherwise identical).
}

impl PluginFactory for ModFactory {
    fn load(&self, _c: &Context, args: &[Value]) -> Result<Value, TemplateException> {
        Ok(Value::object(Mod {
            what: self.what,
            value: args.first().cloned().unwrap_or(Value::Undefined),
        }))
    }
}

/// Template::Plugin::Simple (a Template::Plugin::Filter subclass).
struct SimpleFactory;

impl PluginFactory for SimpleFactory {
    fn load(&self, c: &Context, _args: &[Value]) -> Result<Value, TemplateException> {
        let f: template_toolkit::filters::DynamicFilterFactory = Rc::new(|_args| {
            let ff: template_toolkit::filters::FilterFn =
                Rc::new(|t: &str| Ok(format!("**{}**", t)));
            Ok(FilterFactoryOutcome::Ok(ff))
        });
        c.define_filter("simple", FilterDef::Dynamic(f));
        Ok(Value::hash(HashMap::new()))
    }
}

/// Plugin that is not loadable (Perl: deleted from STD_PLUGINS / not found in PLUGIN_BASE).
struct Missing(&'static str);

impl PluginFactory for Missing {
    fn load(&self, _c: &Context, _a: &[Value]) -> Result<Value, TemplateException> {
        Err(TemplateException::new(
            "plugin",
            format!("{}: plugin not found", self.0),
        ))
    }
}

fn fac(what: &'static str) -> Rc<dyn PluginFactory> {
    Rc::new(ModFactory { what })
}

#[test]
fn plugins() {
    let src = format!(
        "{}/Template-Toolkit-3.009/t/test/lib",
        env!("CARGO_MANIFEST_DIR")
    );
    let inc = || {
        vec![template_toolkit::provider::IncludePathItem::Path(
            src.clone(),
        )]
    };

    // PLUGIN_BASE => ['MyPlugs','Template::Plugin']: Perl modules cannot be
    // searched, so the modules found through the base are registered by name.
    let mut p1: HashMap<String, Rc<dyn PluginFactory>> = HashMap::new();
    p1.insert("Foo".into(), fac("Foo plugin"));
    p1.insert("Bar".into(), fac("Bar plugin"));
    p1.insert("Simple".into(), Rc::new(SimpleFactory));
    let tt1 = Template::builder()
        .plugin_base(vec!["MyPlugs".into(), "Template::Plugin".into()])
        .plugins(p1)
        .include_path(inc())
        .build()
        .unwrap();

    let mut p2: HashMap<String, Rc<dyn PluginFactory>> = HashMap::new();
    p2.insert("bar".into(), fac("Bar plugin"));
    p2.insert("baz".into(), fac("Foo plugin"));
    p2.insert("cgi".into(), fac("Bar plugin"));
    let tt2 = Template::builder().plugins(p2).build().unwrap();

    // LOAD_PERL => 1: MyPlugs.Baz resolves to the MyPlugs::Baz module
    let mut p3: HashMap<String, Rc<dyn PluginFactory>> = HashMap::new();
    p3.insert("MyPlugs.Baz".into(), fac("Baz module"));
    let tt3 = Template::builder().plugins(p3).build().unwrap();

    // PLUGIN_BASE => 'MyPlugs' without the default base, and 'date' deleted
    // from STD_PLUGINS (global mutation in the Perl test).
    let mut p4: HashMap<String, Rc<dyn PluginFactory>> = HashMap::new();
    p4.insert("Foo".into(), fac("Foo plugin"));
    p4.insert("Date".into(), Rc::new(Missing("Date")));
    let tt4 = Template::builder()
        .plugin_base(vec!["MyPlugs".into()])
        .plugins(p4)
        .build()
        .unwrap();
    let tt5 = Template::builder().build().unwrap();

    test_expect(
        include_str!("corpus/plugins.tt"),
        vec![
            ("def", Template::new()),
            ("tt1", tt1),
            ("tt2", tt2),
            ("tt3", tt3),
            ("tt4", tt4),
            ("tt5", tt5),
        ],
        callsign(),
    );
}
