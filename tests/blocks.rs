mod common;
use common::*;
use template_toolkit::{provider::IncludePathItem, value::Value, Template};

#[test]
fn blocks() {
    let lib = "corpora/Template-Toolkit-3.009/t/test/lib";
    let cwd = std::env::current_dir().unwrap();
    let dir = format!("{}/{}", cwd.display(), lib);
    let mk = |expose: bool| {
        Template::builder()
            .include_path(vec![IncludePathItem::Path(lib.into())])
            .expose_blocks(expose)
            .absolute(true)
            .build()
            .unwrap()
    };
    let v = vars(vec![
        ("a", "alpha".into()),
        ("b", "bravo".into()),
        ("dir", Value::Str(dir)),
    ]);
    test_expect(
        &tt_test("blocks"),
        vec![("off", mk(false)), ("on", mk(true))],
        v,
    );
}
