use template_toolkit::{provider::IncludePathItem, Template};

const FILE: &str = "corpora/Template-Toolkit-3.009/t/test/dir/file1";

fn run(tt: Template, f: &str) -> Option<String> {
    tt.process_file(f, None).ok()
}

#[test]
fn process_relative() {
    let dot = || vec![IncludePathItem::Path(".".into())];
    let expect = Some("This is file 1".to_string());
    let dotf = format!("./{}", FILE);

    for f in [FILE, dotf.as_str()] {
        let tt = Template::builder()
            .include_path(dot())
            .relative(true)
            .build()
            .unwrap();
        assert_eq!(run(tt, f), expect, "INCLUDE_PATH='.' RELATIVE: {}", f);
    }
    for f in [FILE, dotf.as_str()] {
        let tt = Template::builder().relative(true).build().unwrap();
        assert_eq!(run(tt, f), expect, "RELATIVE: {}", f);
    }
    let tt = Template::builder().include_path(dot()).build().unwrap();
    assert_eq!(run(tt, FILE), expect, "INCLUDE_PATH='.'");

    let tt = Template::builder().include_path(dot()).build().unwrap();
    assert_eq!(run(tt, &dotf), None, "fails without RELATIVE");

    assert_eq!(run(Template::new(), FILE), expect, "no INCLUDE_PATH");
    assert_eq!(run(Template::new(), &dotf), None, "./ without INCLUDE_PATH");
}
