mod common;
use std::{cell::RefCell, fs, path::Path, rc::Rc};

use common::*;
use template_toolkit::{
    parser::Parser,
    provider::{set_max_dirs, FetchResult, IncludePathItem, Provider},
    value::{CallContext, MethodResult, TemplateObject, Value},
    Template,
};

fn delivered(p: &Provider, file: &str) -> bool {
    matches!(p.fetch(file), FetchResult::Ok(_))
}
fn declined(p: &Provider, file: &str) -> bool {
    matches!(p.fetch(file), FetchResult::Declined)
}
fn denied(p: &Provider, file: &str) -> bool {
    matches!(p.fetch(file), FetchResult::Error(_))
}

/// package My::DPaths: an object with a paths() method.
struct DPaths(Vec<String>);
impl TemplateObject for DPaths {
    fn type_name(&self) -> &str {
        "My::DPaths"
    }
    fn call_method(&self, method: &str, _a: &[Value], _c: CallContext) -> MethodResult {
        match method {
            "paths" => MethodResult::Ok(Value::list(
                self.0.iter().map(|s| Value::Str(s.clone())).collect(),
            )),
            _ => MethodResult::NotFound,
        }
    }
}

#[test]
fn provider() {
    let root = env!("CARGO_MANIFEST_DIR");
    let dir = "Template-Toolkit-3.009/t/test/src".to_string();
    let lib = "Template-Toolkit-3.009/t/test/lib".to_string();
    let file = "foo";
    let relfile = format!("./{}/{}", dir, file);
    let absfile = format!("{}/{}/{}", root, dir, file);
    let newfile = format!("{}/foobar", dir);

    let update_file = {
        let newfile = newfile.clone();
        move |content: &str| {
            std::thread::sleep(std::time::Duration::from_secs(2)); // ensure file time stamps differ
            fs::write(&newfile, content).expect("write newfile");
        }
    };
    update_file("This is the old content");

    let parser = Rc::new(RefCell::new(
        Parser::builder().post_chomp(1).build().unwrap(),
    ));
    let provinc = Provider::builder()
        .include_path(vec![IncludePathItem::Path(dir.clone())])
        .parser(parser.clone())
        .tolerant(true)
        .build()
        .unwrap();
    let provabs = Provider::builder()
        .absolute(true)
        .parser(parser.clone())
        .build()
        .unwrap();
    let provrel = Provider::builder()
        .relative(true)
        .parser(parser.clone())
        .build()
        .unwrap();
    assert!(Rc::ptr_eq(&provinc.parser, &provabs.parser));
    assert!(Rc::ptr_eq(&provabs.parser, &provrel.parser));

    // matrix
    assert!(delivered(&provinc, file));
    assert!(declined(&provinc, &absfile));
    assert!(declined(&provinc, &relfile));
    assert!(declined(&provabs, file));
    assert!(delivered(&provabs, &absfile));
    assert!(denied(&provabs, &relfile));
    assert!(declined(&provrel, file));
    assert!(denied(&provrel, &absfile));
    assert!(delivered(&provrel, &relfile));

    // Perl passes an open file handle (\*GLOBFILE) as the template; Rust has no
    // file handle values, so the handle's contents are given as template text.
    let ttglob = Template::new();
    let globtext = fs::read_to_string(format!("{}/{}/baz", root, dir)).unwrap();
    let out = ttglob
        .process_text(&globtext, Some(vars(vec![("a", "globtest".into())])))
        .unwrap();
    assert_eq!(
        out, "This is the baz file, a: globtest\n",
        "Fetch template from file handle"
    );

    let with = |p: Provider| {
        Template::builder()
            .load_templates(vec![Rc::new(RefCell::new(p))])
            .build()
            .unwrap()
    };
    let ttinc = with(provinc);
    let ttabs = with(provabs);
    let ttrel = with(provrel);

    let dpaths = {
        let lib = lib.clone();
        move || {
            Ok(vec![
                IncludePathItem::Path(format!("{}/one", lib)),
                IncludePathItem::Path(format!("{}/two", lib)),
            ])
        }
    };
    set_max_dirs(42);

    fn badpaths() -> Result<Vec<IncludePathItem>, String> {
        Ok(vec![IncludePathItem::Generator(Rc::new(badpaths))])
    }
    let obj = DPaths(vec![format!("{}/two", lib), format!("{}/one", lib)]);

    let mk = |items: Vec<IncludePathItem>| {
        Template::builder()
            .include_path(items)
            .parser(parser.borrow().clone())
            .build()
            .unwrap()
    };
    let ttd1 = mk(vec![
        IncludePathItem::Generator(Rc::new(dpaths)),
        IncludePathItem::Path(dir.clone()),
    ]);
    let ttd2 = mk(vec![
        IncludePathItem::Object(Rc::new(obj)),
        IncludePathItem::Path(dir.clone()),
    ]);
    let ttd3 = mk(vec![IncludePathItem::Generator(Rc::new(badpaths))]);

    let fixfile = Value::function(move |args, _| {
        update_file(&args.iter().map(|v| v.to_tt_string()).collect::<String>());
        Ok(Value::Str(String::new()))
    });
    let params = vars(vec![
        ("file", file.into()),
        ("relfile", relfile.as_str().into()),
        ("absfile", absfile.as_str().into()),
        ("fixfile", fixfile),
    ]);
    test_expect(
        include_str!("corpus/provider.tt"),
        vec![
            ("ttinc", ttinc),
            ("ttabs", ttabs),
            ("ttrel", ttrel),
            ("ttd1", ttd1),
            ("ttd2", ttd2),
            ("ttdbad", ttd3),
        ],
        params,
    );
    let _ = Path::new(&newfile);
}
