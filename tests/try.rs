mod common;
use common::*;
use template_toolkit::{
    constants::*,
    error::{ControlFlow, TemplateException},
    provider::IncludePathItem,
    value::Value,
    Template,
};

#[test]
fn try_() {
    let mut v = callsign();
    v.insert(
        "throw_egg".into(),
        Value::function(|_, _| {
            Err(ControlFlow::Exception(TemplateException::new(
                "egg",
                "scrambled",
            )))
        }),
    );
    v.insert(
        "throw_any".into(),
        Value::function(|_, _| {
            // die "undefined error\n"
            Err(ControlFlow::Exception(TemplateException::new(
                "undef",
                "undefined error\n",
            )))
        }),
    );
    let tt = Template::builder()
        .include_path(vec![
            IncludePathItem::Path("corpora/Template-Toolkit-3.009/t/test/lib".into()),
            IncludePathItem::Path("test/lib".into()),
        ])
        .post_chomp(CHOMP_ONE)
        .build()
        .unwrap();
    test_expect(&tt_test("try"), vec![("default", tt)], v);
}
