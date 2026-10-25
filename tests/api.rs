//! The registration API: filters, filter factories, plugins, vmethods,
//! functions and variables, as closures and as trait implementations.

use std::{cell::Cell, rc::Rc};

use template_toolkit::{
    context::{with_context, Context},
    error::{ControlFlow, TemplateException},
    filters::{Filter, FilterFactoryOutcome, FilterFn},
    value::{CallContext, Value},
    Template,
};

fn render(tt: &Template, src: &str) -> String {
    tt.process_text(src, None)
        .unwrap_or_else(|e| panic!("{src}: {e}"))
}

/// A filter as a type, with its own configuration.
struct Wrap {
    left: &'static str,
    right: &'static str,
}

impl Filter for Wrap {
    fn filter(&self, text: &str) -> Result<String, ControlFlow> {
        Ok(format!("{}{}{}", self.left, text, self.right))
    }
}

#[test]
fn filters() {
    let tt = Template::new();
    tt.define_filter("shout", |t: &str| Ok(t.to_uppercase()))
        .define_filter(
            "brackets",
            Wrap {
                left: "[",
                right: "]",
            },
        );
    assert_eq!(render(&tt, "[% 'hi' | shout | brackets %]"), "[HI]");
    assert_eq!(render(&tt, "[% FILTER shout %]block[% END %]"), "BLOCK");
}

#[test]
fn filter_factories() {
    let tt = Template::new();
    // Context-free: configured only by its arguments.
    tt.define_filter_factory("times", |args: &[Value]| {
        let n = args.first().map(|v| v.to_int()).unwrap_or(1).max(0) as usize;
        let f: FilterFn = Rc::new(move |t: &str| Ok(t.repeat(n)));
        Ok(FilterFactoryOutcome::Ok(f))
    });
    // Context-aware: reads a template variable when the filter is created.
    tt.define_variable("prefix", "> ");
    tt.define_filter_factory(
        "quote",
        with_context(|ctx: &Context, _args: &[Value]| {
            let prefix = ctx
                .stash()
                .get(&Value::from("prefix"), None)?
                .to_tt_string();
            let f: FilterFn = Rc::new(move |t: &str| {
                Ok(t.lines()
                    .map(|l| format!("{prefix}{l}"))
                    .collect::<Vec<_>>()
                    .join("\n"))
            });
            Ok(FilterFactoryOutcome::Ok(f))
        }),
    );
    assert_eq!(render(&tt, "[% 'ab' | times(3) %]"), "ababab");
    assert_eq!(
        render(&tt, "[% FILTER quote %]one\ntwo[% END %]"),
        "> one\n> two"
    );
    // Errors thrown by a factory become 'filter' exceptions, as in Perl.
    tt.define_filter_factory("broken", |_: &[Value]| {
        Err(ControlFlow::Exception(TemplateException::new("", "no way")))
    });
    assert_eq!(
        render(
            &tt,
            "[% TRY %][% 'x' | broken %][% CATCH %][% error.type %]: [% error.info %][% END %]"
        ),
        "filter: no way"
    );
}

#[test]
fn plugins() {
    let tt = Template::new();
    tt.define_plugin("Greeter", |args: &[Value]| {
        let greeting = args
            .first()
            .map(|v| v.to_tt_string())
            .unwrap_or_else(|| "Hello".into());
        Ok(Value::function(move |a: &[Value], _| {
            Ok(Value::Str(format!(
                "{greeting}, {}!",
                a.first().map(|v| v.to_tt_string()).unwrap_or_default()
            )))
        }))
    });
    assert_eq!(
        render(&tt, "[% USE g = Greeter('Hi') %][% g('Ada') %]"),
        "Hi, Ada!"
    );
}

#[test]
fn vmethods_capture_state() {
    let tt = Template::new();
    let calls = Rc::new(Cell::new(0));
    let counter = Rc::clone(&calls);
    tt.define_vmethod("scalar", "rot13", move |v: &Value, _: &[Value]| {
        counter.set(counter.get() + 1);
        Ok(Value::Str(
            v.to_tt_string()
                .chars()
                .map(|c| match c {
                    'a'..='z' => (((c as u8 - b'a') + 13) % 26 + b'a') as char,
                    'A'..='Z' => (((c as u8 - b'A') + 13) % 26 + b'A') as char,
                    _ => c,
                })
                .collect(),
        ))
    });
    tt.define_vmethod("list", "total", |v: &Value, _: &[Value]| {
        let sum: i64 = match v {
            Value::List(l) => l.borrow().iter().map(|x| x.to_int()).sum(),
            _ => 0,
        };
        Ok(Value::Int(sum))
    });
    assert_eq!(
        render(&tt, "[% s = 'Hello'; s.rot13 %] [% s.rot13.rot13 %]"),
        "Uryyb Hello"
    );
    assert_eq!(render(&tt, "[% nums = [1, 2, 3]; nums.total %]"), "6");
    assert_eq!(calls.get(), 3);
}

#[test]
fn functions_and_variables() {
    let tt = Template::new();
    tt.define_variable("site", "example.org").define_function(
        "double",
        |args: &[Value], _: CallContext| {
            Ok(Value::Int(
                args.first().map(|v| v.to_int()).unwrap_or(0) * 2,
            ))
        },
    );
    assert_eq!(
        render(&tt, "[% site %]: [% double(21) %]"),
        "example.org: 42"
    );
    // Globals persist across process calls; per-call variables override them.
    let mut vars = std::collections::HashMap::new();
    vars.insert("site".to_string(), Value::from("override"));
    assert_eq!(
        tt.process_text("[% site %]", Some(vars)).unwrap(),
        "override"
    );
    assert_eq!(render(&tt, "[% site %]"), "example.org");
}
