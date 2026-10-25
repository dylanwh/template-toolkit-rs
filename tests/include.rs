mod common;
use std::time::UNIX_EPOCH;

use common::*;
use template_toolkit::{provider::IncludePathItem, value::Value, Template};

#[test]
fn include() {
    let dir = "corpora/Template-Toolkit-3.009/t/test";
    let src = |s: &str| IncludePathItem::Path(format!("{dir}/{s}"));
    let mut replace = vars(vec![
        ("a", "alpha".into()),
        ("b", "bravo".into()),
        (
            "c",
            hash(vec![
                ("d", "delta".into()),
                ("e", "echo".into()),
                ("f", hash(vec![("g", "golf".into()), ("h", "hotel".into())])),
            ]),
        ),
        ("r", "romeo".into()),
        ("s", "sierra".into()),
        ("t", "tango".into()),
    ]);

    let tproc = Template::builder()
        .interpolate(true)
        .include_path(vec![src("src"), src("lib")])
        .trim(true)
        .auto_reset(false)
        .default_template("default".to_string())
        .build()
        .unwrap();

    // $incpath is mutated after construction upstream: [ "$dir/src", '/nowhere' ] -> [ "$dir/src", "$dir/lib" ]
    // The provider copies its path at construction here, so build with the final path.
    let tt_reset = Template::builder()
        .interpolate(true)
        .include_path(vec![src("src"), src("lib")])
        .trim(true)
        .recursion(true)
        .default_template("bad_default".to_string())
        .build()
        .unwrap();

    let output = tproc
        .process_file("metadata", Some(replace.clone()))
        .expect("metadata processed");
    replace.insert("metaout".into(), output.into());
    let modtime = std::fs::metadata(format!("{dir}/src/metadata"))
        .unwrap()
        .modified()
        .unwrap()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    replace.insert("metamod".into(), Value::Int(modtime as i64));

    test_expect(
        &tt_test("include"),
        vec![("default", tproc), ("reset", tt_reset)],
        replace,
    );
}
