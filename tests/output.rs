mod common;
use std::fs;

use common::*;
use template_toolkit::{provider::IncludePathItem, value::Value, Template};

#[test]
fn output() {
    let dir = format!(
        "{}/Template-Toolkit-3.009/t/test",
        env!("CARGO_MANIFEST_DIR")
    );
    let (f1, f2) = ("foo.bar", "foo.baz");
    let file1 = format!("{}/tmp/{}", dir, f1);
    let file2 = format!("{}/tmp/{}", dir, f2);
    let ip = || {
        vec![
            IncludePathItem::Path(format!("{}/src", dir)),
            IncludePathItem::Path(format!("{}/lib", dir)),
        ]
    };
    let want = "This is the foo file, a is alpha";
    let foo = Value::Str("foo".into());

    let _ = fs::remove_file(&file1);
    let tt = Template::builder()
        .include_path(ip())
        .output_path(format!("{}/tmp", dir))
        .build()
        .unwrap();
    tt.process_to_file(&foo, Some(callsign()), f1)
        .expect("process");
    assert!(std::path::Path::new(&file1).is_file());
    assert_eq!(fs::read_to_string(&file1).unwrap().trim_end(), want);
    let _ = fs::remove_file(&file1);

    let _ = fs::remove_file(&file2);
    let tt = Template::builder()
        .include_path(ip())
        .output_path(format!("{}/tmp", dir))
        .output(f2.to_string())
        .build()
        .unwrap();
    tt.process_to_output(&foo, Some(callsign()))
        .expect("process");
    assert!(std::path::Path::new(&file2).is_file());
    assert_eq!(fs::read_to_string(&file2).unwrap().trim_end(), want);
    let _ = fs::remove_file(&file2);

    // Skipped: the My::Template::DEBUG "set binmode" subclass-override checks
    // (binmode option on the output handle is a Perl IO-layer concept).
}
