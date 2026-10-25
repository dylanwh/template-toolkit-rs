mod common;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use common::*;
use template_toolkit::{provider::IncludePathItem, value::Value, Template};

fn copy_dir(from: &Path, to: &Path) {
    let _ = std::fs::remove_dir_all(to);
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        if e.file_type().unwrap().is_dir() {
            copy_dir(&e.path(), &to.join(e.file_name()));
        } else {
            std::fs::copy(e.path(), to.join(e.file_name())).unwrap();
        }
    }
}

// Upstream runs in t/test/src and leaves .ttc files behind; work on a private
// copy instead so the source tree stays clean.
fn setup() -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("compile2_src");
    copy_dir(Path::new("Template-Toolkit-3.009/t/test/src"), &dir);
    dir
}

fn config(dir: &Path) -> Template {
    let zero = dir.join("divisionbyzero").to_string_lossy().to_string();
    let consts: HashMap<String, Value> = [("zero".to_string(), Value::Str(zero))].into();
    Template::builder()
        .post_chomp(1)
        .include_path(vec![IncludePathItem::Path(
            dir.to_string_lossy().to_string(),
        )])
        .compile_ext(".ttc".to_string())
        .constants(consts)
        .build()
        .unwrap()
}

#[test]
fn compile2() {
    let dir = setup();
    // Precondition (upstream: left behind by compile1.t): compiled foo/complex/divisionbyzero.
    let pre = config(&dir);
    for n in ["foo", "complex", "divisionbyzero"] {
        let _ = pre.process_file(n, None);
    }
    let compiled = dir.join("foo.ttc");
    assert!(compiled.is_file());
    assert!(dir.join("complex.ttc").is_file());

    let tt = config(&dir);
    let out = tt
        .process_file("baz", Some(vars(vec![("showname", 1.into())])))
        .unwrap();
    assert!(out.starts_with("name: baz"), "{:?}", out);

    // Hack the compiled file, preserving its mtime, to prove it is read from cache.
    let mtime = std::fs::metadata(&compiled).unwrap().modified().unwrap();
    let foo = std::fs::read_to_string(&compiled).unwrap();
    assert!(foo.contains("the foo file"));
    std::fs::write(
        &compiled,
        foo.replace("the foo file", "the hacked foo file"),
    )
    .unwrap();
    std::fs::File::options()
        .write(true)
        .open(&compiled)
        .unwrap()
        .set_modified(mtime)
        .unwrap();

    test_expect(
        include_str!("corpus/compile2.tt"),
        vec![("default", config(&dir))],
        vars(vec![]),
    );
}
