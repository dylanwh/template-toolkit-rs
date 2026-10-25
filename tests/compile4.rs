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

// Upstream works in t/test (src/, tmp/cache); use a private copy of src/.
fn setup() -> (PathBuf, PathBuf) {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("compile4_test");
    let _ = std::fs::remove_dir_all(&root);
    copy_dir(
        Path::new("corpora/Template-Toolkit-3.009/t/test/src"),
        &root.join("src"),
    );
    let cdir = root.join("tmp").join("cache");
    std::fs::create_dir_all(&cdir).unwrap();
    (root, cdir)
}

fn config(root: &Path, cdir: &str) -> Template {
    let dir = root.to_string_lossy().to_string();
    let zero = root
        .join("src")
        .join("divisionbyzero")
        .to_string_lossy()
        .to_string();
    let consts: HashMap<String, Value> = [
        ("dir".to_string(), Value::Str(dir.clone())),
        ("zero".to_string(), Value::Str(zero)),
    ]
    .into();
    Template::builder()
        .post_chomp(1)
        .include_path(vec![IncludePathItem::Path(format!("{}/src", dir))])
        .compile_dir(cdir.to_string())
        .compile_ext(".ttc".to_string())
        .absolute(true)
        .constants(consts)
        .build()
        .unwrap()
}

#[test]
fn compile4() {
    let (root, cdir) = setup();
    let v = vars(vec![(
        "root",
        Value::Str(root.to_string_lossy().to_string()),
    )]);
    test_expect(
        &tt_test("compile4"),
        vec![("default", config(&root, cdir.to_str().unwrap()))],
        v,
    );
}
