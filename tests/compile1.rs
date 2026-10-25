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
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("compile1_src");
    copy_dir(Path::new("corpora/Template-Toolkit-3.009/t/test/src"), &dir);
    dir
}

#[test]
fn compile1() {
    let dir = setup();
    let zero = dir.join("divisionbyzero").to_string_lossy().to_string();
    let consts: HashMap<String, Value> = [("zero".to_string(), Value::Str(zero))].into();
    // EVAL_PERL is out of scope: the first upstream case ([% INCLUDE evalperl %]) is skipped.
    let corpus = &tt_test("compile1");
    let skip = corpus.find("-- test --").unwrap();
    let skip2 = skip + 1 + corpus[skip + 1..].find("-- test --").unwrap();
    let tt = Template::builder()
        .post_chomp(1)
        .include_path(vec![IncludePathItem::Path(
            dir.to_string_lossy().to_string(),
        )])
        .compile_ext(".ttc".to_string())
        .constants(consts)
        .build()
        .unwrap();
    test_expect(&corpus[skip2..], vec![("default", tt)], vars(vec![]));
    assert!(dir.join("foo.ttc").is_file());
    assert!(dir.join("complex.ttc").is_file());
}
