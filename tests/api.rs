//! The registration API: filters, filter factories, plugins, vmethods,
//! functions and variables, as typed closures, context-aware closures and
//! trait implementations.

use std::{cell::Cell, collections::HashMap, rc::Rc};

use template_toolkit::{
    convert::Rest,
    error::ControlFlow,
    filters::Filter,
    value::{CallContext, Value},
    Template,
};

fn render(tt: &Template, src: &str) -> String {
    tt.process_text(src, None)
        .unwrap_or_else(|e| panic!("{src}: {e}"))
}

fn render_err(tt: &Template, src: &str) -> String {
    match tt.process_text(src, None) {
        Ok(out) => panic!("{src}: expected an error, got {out:?}"),
        Err(e) => e.to_string(),
    }
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
    // Closure parameter and return types are inferred.
    tt.define_filter("shout", |t| t.to_uppercase())
        .define_filter("nonempty", |t| {
            if t.is_empty() {
                Err("empty!".into())
            } else {
                Ok(t.to_string())
            }
        })
        .define_filter_impl(
            "brackets",
            Wrap {
                left: "[",
                right: "]",
            },
        );
    assert_eq!(render(&tt, "[% 'hi' | shout | brackets %]"), "[HI]");
    assert_eq!(render(&tt, "[% FILTER shout %]block[% END %]"), "BLOCK");
    assert_eq!(
        render_err(&tt, "[% '' | nonempty %]"),
        "undef error - empty!"
    );
}

#[test]
fn filter_factories() {
    let tt = Template::new();
    // Typed arguments; the returned filter is any `Fn(&str) -> String`.
    tt.define_filter_factory("times", |n: Option<usize>| {
        let n = n.unwrap_or(2);
        move |t: &str| t.repeat(n)
    });
    // The context form, as Perl's dynamic filters get: read a variable.
    tt.define_variable("prefix", "> ");
    tt.define_filter_factory_with_context("quote", |ctx, _args| {
        let prefix = ctx
            .stash()
            .get(&Value::from("prefix"), None)?
            .to_tt_string();
        Ok(move |t: &str| {
            t.lines()
                .map(|l| format!("{prefix}{l}"))
                .collect::<Vec<_>>()
                .join("\n")
        })
    });
    assert_eq!(
        render(&tt, "[% 'ab' | times(3) %] [% 'ab' | times %]"),
        "ababab abab"
    );
    assert_eq!(
        render(&tt, "[% FILTER quote %]one\ntwo[% END %]"),
        "> one\n> two"
    );
    // Errors from a factory become 'filter' exceptions, as in Perl.
    tt.define_filter_factory("broken", || -> Result<fn(&str) -> String, ControlFlow> {
        Err("no way".into())
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
    tt.define_plugin("Greeter", |greeting: Option<String>| {
        let greeting = greeting.unwrap_or_else(|| "Hello".into());
        Value::function(move |a: &[Value], _| {
            Ok(Value::Str(format!("{greeting}, {}!", a[0].to_tt_string())))
        })
    });
    tt.define_plugin_with_context("Here", |ctx, _args| {
        ctx.stash()
            .get(&Value::from("site"), None)
            .map(|v| format!("on {}", v.to_tt_string()))
    });
    tt.define_variable("site", "example.org");
    assert_eq!(
        render(&tt, "[% USE g = Greeter('Hi') %][% g('Ada') %]"),
        "Hi, Ada!"
    );
    assert_eq!(render(&tt, "[% USE h = Here %][% h %]"), "on example.org");
}

#[test]
fn vmethods() {
    let tt = Template::new();
    let calls = Rc::new(Cell::new(0));
    let counter = Rc::clone(&calls);
    // Typed: the target value is the first argument; closures may capture state.
    tt.define_vmethod("scalar", "rot13", move |s: String| {
        counter.set(counter.get() + 1);
        s.chars()
            .map(|c| match c {
                'a'..='z' => (((c as u8 - b'a') + 13) % 26 + b'a') as char,
                'A'..='Z' => (((c as u8 - b'A') + 13) % 26 + b'A') as char,
                _ => c,
            })
            .collect::<String>()
    });
    tt.define_vmethod("list", "total", |nums: Vec<f64>| nums.iter().sum::<f64>());
    tt.define_vmethod(
        "scalar",
        "pad",
        |s: String, width: Option<usize>, fill: Option<String>| {
            let fill = fill.unwrap_or_else(|| " ".into());
            let n = width.unwrap_or(8).saturating_sub(s.chars().count());
            format!("{}{s}", fill.repeat(n))
        },
    );
    // Raw form over (&Value, &[Value]).
    tt.define_vmethod_impl("hash", "count", |v: &Value, _: &[Value]| match v {
        Value::Hash(h) => Ok(Value::Int(h.borrow().len() as i64)),
        _ => Ok(Value::Int(0)),
    });
    assert_eq!(
        render(&tt, "[% s = 'Hello'; s.rot13 %] [% s.rot13.rot13 %]"),
        "Uryyb Hello"
    );
    assert_eq!(calls.get(), 3);
    assert_eq!(render(&tt, "[% nums = [1, '2', 3.5]; nums.total %]"), "6.5");
    assert_eq!(
        render(&tt, "[% x = 'x' %][[% x.pad(4) %]] [[% x.pad(3, '.') %]]"),
        "[   x] [..x]"
    );
    assert_eq!(render(&tt, "[% h = { a => 1, b => 2 }; h.count %]"), "2");
}

#[test]
fn typed_functions() {
    let tt = Template::new();
    tt.define_function("add", |a: i64, b: i64| a + b)
        .define_function("greet", |name: Option<String>| {
            format!("Hello, {}!", name.as_deref().unwrap_or("you"))
        })
        .define_function("sum", |label: String, nums: Rest<f64>| {
            format!("{label}={}", nums.0.iter().sum::<f64>())
        })
        .define_function("keys", |h: HashMap<String, Value>| {
            let mut k: Vec<String> = h.into_keys().collect();
            k.sort();
            k
        })
        .define_function("is_even", |n: i64| n % 2 == 0)
        .define_function("checked_div", |a: i64, b: i64| {
            if b == 0 {
                Err("division by zero".into())
            } else {
                Ok(a / b)
            }
        });

    // Perl-style coercion: numeric strings are numbers, missing args are undefined.
    assert_eq!(render(&tt, "[% add('3', 4) %] [% add(2) %]"), "7 2");
    assert_eq!(
        render(&tt, "[% greet('Ada') %] [% greet %]"),
        "Hello, Ada! Hello, you!"
    );
    assert_eq!(render(&tt, "[% sum('total', 1, 2, 3.5) %]"), "total=6.5");
    assert_eq!(
        render(&tt, "[% keys({ b => 1, a => 2 }).join(',') %]"),
        "a,b"
    );
    assert_eq!(
        render(&tt, "[% is_even(4) ? 'even' : 'odd' %] [[% is_even(3) %]]"),
        "even []"
    );
    assert_eq!(render(&tt, "[% checked_div(7, 2) %]"), "3");
    assert_eq!(
        render_err(&tt, "[% checked_div(1, 0) %]"),
        "undef error - division by zero"
    );
    // Structural mismatches report the argument position and function name.
    assert_eq!(
        render_err(&tt, "[% keys([1, 2]) %]"),
        "undef error - bad argument #1 to `keys`: expected hash, got ARRAY"
    );
}

#[test]
fn raw_functions_and_variables() {
    let tt = Template::new();
    tt.define_variable("site", "example.org")
        .define_function_raw("argc", |args, _cx: CallContext| args.len());
    assert_eq!(
        render(&tt, "[% site %]: [% argc(1, 2, 3) %]"),
        "example.org: 3"
    );
    // Globals persist across process calls; per-call variables override them.
    let mut vars = HashMap::new();
    vars.insert("site".to_string(), Value::from("override"));
    assert_eq!(
        tt.process_text("[% site %]", Some(vars)).unwrap(),
        "override"
    );
    assert_eq!(render(&tt, "[% site %]"), "example.org");
}
