use std::{cell::RefCell, collections::HashMap, rc::Rc};

use template_toolkit::{
    context::Context,
    iterator::TemplateIterator as TtIterator,
    parser::Parser,
    provider::{IncludePathItem, Provider},
    service::Service,
    stash::Stash,
    value::Value,
    Template,
};

// Port of t/config.t.  Template::Config is a Perl module factory; the Rust
// equivalents are the bon builders for each component.  Checks that inspect
// Perl hash internals are mapped onto the public Rust fields/accessors.
// PLUGIN_BASE / LOAD_PERL (Perl package search) and instdir() have no Rust
// equivalent and are skipped.

fn paths(p: &Provider) -> Vec<String> {
    p.include_path
        .iter()
        .map(|i| match i {
            IncludePathItem::Path(s) => s.clone(),
            _ => panic!("unexpected include path item"),
        })
        .collect()
}

#[test]
fn parser() {
    let p = Parser::builder()
        .pre_chomp(1)
        .interpolate(true)
        .build()
        .unwrap();
    assert_eq!(p.current_style().pre_chomp, 1);
    assert!(p.current_style().interpolate);
    let p = Parser::builder().post_chomp(1).build().unwrap();
    assert_eq!(p.current_style().post_chomp, 1);
}

#[test]
fn provider() {
    let parser = Rc::new(RefCell::new(
        Parser::builder().post_chomp(1).build().unwrap(),
    ));
    let p = Provider::builder()
        .include_path_str("here:there".to_string())
        .parser(parser)
        .build()
        .unwrap();
    assert_eq!(paths(&p).join("..."), "here...there");
    assert_eq!(p.parser.borrow().current_style().post_chomp, 1);

    // ANYCASE / INTERPOLATE given to the Template reach the provider's parser.
    let tt = Template::builder()
        .anycase(true)
        .interpolate(true)
        .build()
        .unwrap();
    let provs = tt.context().providers();
    let style = provs[0].borrow().parser.borrow().current_style().clone();
    assert!(style.anycase);
    assert!(style.interpolate);

    let p = Provider::builder()
        .include_path_str("cat:mat".to_string())
        .build()
        .unwrap();
    assert_eq!(paths(&p).join("..."), "cat...mat");
    let text = "The cat sat on the mat";
    assert!(matches!(
        p.fetch_text(text, None),
        template_toolkit::provider::FetchResult::Ok(_)
    ));
}

#[test]
fn stash() {
    let m: HashMap<String, Value> = [
        ("foo".to_string(), Value::Int(10)),
        ("bar".to_string(), Value::Int(20)),
    ]
    .into();
    let s = Stash::new(Some(m));
    assert_eq!(s.get_str("foo").unwrap().to_number(), 10.0);
    assert_eq!(s.get_str("bar").unwrap().to_number(), 20.0);
    let m: HashMap<String, Value> = [
        ("foo".to_string(), Value::Int(30)),
        (
            "bar".to_string(),
            Value::function(|_, _| Ok(Value::Str("forty".into()))),
        ),
    ]
    .into();
    let s = Stash::new(Some(m));
    assert_eq!(s.get_str("foo").unwrap().to_number(), 30.0);
    assert_eq!(s.get_str("bar").unwrap().to_tt_string(), "forty");
}

#[test]
fn context() {
    let c = Context::builder().build().unwrap();
    let _ = c;
    let tt = Template::builder()
        .include_path(vec![IncludePathItem::Path("anywhere".into())])
        .build()
        .unwrap();
    let provs = tt.context().providers();
    assert_eq!(paths(&provs[0].borrow())[0], "anywhere");

    let prov = Rc::new(RefCell::new(
        Provider::builder()
            .parser(Rc::new(RefCell::new(
                Parser::builder().interpolate(true).build().unwrap(),
            )))
            .build()
            .unwrap(),
    ));
    let m: HashMap<String, Value> = [("foo".to_string(), Value::Int(30))].into();
    let c = Context::builder()
        .load_templates(vec![prov])
        .stash(Stash::new(Some(m)))
        .build()
        .unwrap();
    assert_eq!(c.stash().get_str("foo").unwrap().to_number(), 30.0);
    assert!(
        c.providers()[0]
            .borrow()
            .parser
            .borrow()
            .current_style()
            .interpolate
    );
}

#[test]
fn service() {
    let tt = Template::builder()
        .include_path(vec![IncludePathItem::Path("amsterdam".into())])
        .build()
        .unwrap();
    let svc: &Service = tt.service();
    let provs = svc.context().providers();
    assert_eq!(paths(&provs[0].borrow())[0], "amsterdam");
}

#[test]
fn iterator() {
    let it = TtIterator::new(&Value::list(vec!["foo".into(), "bar".into(), "baz".into()]));
    assert_eq!(it.get_first().0.to_tt_string(), "foo");
    assert_eq!(it.get_next().0.to_tt_string(), "bar");
    assert_eq!(it.get_next().0.to_tt_string(), "baz");
}
