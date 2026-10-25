use std::{collections::HashMap, rc::Rc};

use template_toolkit::{
    context::CompiledTemplate,
    provider::IncludePathItem,
    value::{CallContext, MethodResult, Value},
    Template,
};

// Port of t/context.t.  EVAL_PERL (the $ttperl engine) is out of scope and
// skipped; the Rust Context returns Rc<Document> for named templates so the
// block-stack checks compare by Rc identity / rendered text instead of Perl
// string equality.

fn mk() -> Template {
    let dir = "Template-Toolkit-3.009/t/test";
    Template::builder()
        .include_path(vec![
            IncludePathItem::Path(format!("{}/src", dir)),
            IncludePathItem::Path(format!("{}/lib", dir)),
        ])
        .trim(true)
        .post_chomp(1)
        .build()
        .unwrap()
}

#[test]
fn context() {
    let tt = mk();
    let context = tt.service().context().clone();
    assert!(context.trim());
    assert!(!context.eval_perl());

    // template()
    let template = context.template("header").expect("header template");
    assert!(!format!("{:?}", template).is_empty());
    let err = context.template("no_such_template").expect_err("error");
    assert_eq!(err.to_string(), "file error - no_such_template: not found");

    let code = Value::function(|_, _| Ok(Value::Str("this is a hard-coded template".into())));
    match context.template_item(&code).unwrap() {
        CompiledTemplate::Closure { func, .. } => {
            assert_eq!(func(&context).unwrap(), "this is a hard-coded template");
        }
        _ => panic!("expected closure for coderef template"),
    }

    // block stack: visit / leave / reset
    let b1 = context.compile_text("hello").unwrap();
    let b2 = context.compile_text("world").unwrap();
    let blocks1: HashMap<String, Rc<_>> = [("some_block_1".to_string(), b1.clone())].into();
    let blocks2: HashMap<String, Rc<_>> = [("some_block_2".to_string(), b2.clone())].into();
    assert!(context.template("some_block_1").is_err());
    context.visit(blocks1.clone());
    assert!(Rc::ptr_eq(&context.template("some_block_1").unwrap(), &b1));
    assert!(context.template("some_block_2").is_err());
    context.visit(blocks2.clone());
    assert!(Rc::ptr_eq(&context.template("some_block_1").unwrap(), &b1));
    assert!(Rc::ptr_eq(&context.template("some_block_2").unwrap(), &b2));
    context.leave();
    assert!(Rc::ptr_eq(&context.template("some_block_1").unwrap(), &b1));
    assert!(context.template("some_block_2").is_err());
    context.leave();
    assert!(context.template("some_block_1").is_err());
    assert!(context.template("some_block_2").is_err());
    context.visit(blocks1);
    assert!(context.template("some_block_1").is_ok());
    assert!(context.template("some_block_2").is_err());
    context.visit(blocks2);
    assert!(context.template("some_block_1").is_ok());
    assert!(context.template("some_block_2").is_ok());
    context.reset();
    assert!(context.template("some_block_1").is_err());
    assert!(context.template("some_block_2").is_err());

    // plugin()
    let opts = Value::hash([("rows".to_string(), Value::Int(2))].into());
    let data = Value::list(vec![1.into(), 2.into(), 3.into(), 4.into()]);
    let plugin = context
        .plugin("Table", &[data, opts])
        .expect("table plugin");
    let Value::Object(obj) = &plugin else {
        panic!("plugin not an object: {:?}", plugin)
    };
    let MethodResult::Ok(row) = obj.call_method("row", &[Value::Int(0)], CallContext::Scalar)
    else {
        panic!("row(0) failed")
    };
    let Value::List(row) = row else {
        panic!("row not a list")
    };
    assert_eq!(row.borrow()[0].to_number(), 1.0);
    assert_eq!(row.borrow()[1].to_number(), 3.0);
    let err = context.plugin("no_such_plugin", &[]).expect_err("error");
    assert_eq!(
        err.to_string(),
        "plugin error - no_such_plugin: plugin not found"
    );

    // filter()
    let f = context.filter("html", None, None).expect("html filter");
    assert_eq!(f.filter("<input/>").unwrap(), "&lt;input/&gt;");
    let args = [Value::from("foo"), Value::from("bar")];
    let f = context
        .filter("replace", Some(&args), Some("repsave"))
        .expect("replace filter");
    assert_eq!(
        f.filter("this is foo, so it is").unwrap(),
        "this is bar, so it is"
    );
    let f = context.filter("repsave", None, None).expect("saved filter");
    assert_eq!(
        f.filter("this is foo, so it is").unwrap(),
        "this is bar, so it is"
    );

    // include() / process()
    let context = tt.context().clone();
    let stash = context.stash();
    stash.set_str("a", "alpha".into(), false).unwrap();
    assert_eq!(stash.get_str("a").unwrap().to_tt_string(), "alpha");
    let a = |s: &str| -> Option<HashMap<String, Value>> {
        Some([("a".to_string(), Value::from(s))].into())
    };
    assert_eq!(
        context.include("baz", None).ok().unwrap(),
        "This is the baz file, a: alpha"
    );
    assert_eq!(
        context.include("baz", a("bravo")).ok().unwrap(),
        "This is the baz file, a: bravo"
    );
    assert_eq!(stash.get_str("a").unwrap().to_tt_string(), "alpha");
    assert_eq!(
        context.process("baz", None).ok().unwrap(),
        "This is the baz file, a: alpha"
    );
    assert_eq!(stash.get_str("a").unwrap().to_tt_string(), "charlie");
    assert_eq!(
        context.process("baz", a("bravo")).ok().unwrap(),
        "This is the baz file, a: bravo"
    );
    assert_eq!(stash.get_str("a").unwrap().to_tt_string(), "charlie");
}
