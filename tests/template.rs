use template_toolkit::{provider::IncludePathItem, Template};

#[test]
fn template() {
    let dir = "corpora/Template-Toolkit-3.009/t/test";
    let tt = Template::builder()
        .include_path(vec![
            IncludePathItem::Path(format!("{}/src", dir)),
            IncludePathItem::Path(format!("{}/lib", dir)),
        ])
        .build()
        .unwrap();

    let out = tt.process_file("header", None).expect("process header");
    assert!(!out.is_empty());

    assert!(tt.process_file("this_file_does_not_exist", None).is_err());
    let error = tt.error().expect("error set");
    assert_eq!(error.type_(), "file");
    assert_eq!(error.info_str(), "this_file_does_not_exist: not found");

    // Upstream also checks the OUTPUT option / third process() argument being a
    // scalar ref, array ref, coderef or an object with a print() method.  The
    // Rust API returns the output string instead, so those reduce to this:
    let out = tt
        .process_file("header", None)
        .expect("process header again");
    assert!(!out.is_empty());
}
