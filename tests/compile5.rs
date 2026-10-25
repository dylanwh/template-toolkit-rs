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
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("compile5_test");
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

fn hack(path: &Path, from: &str, to: &str) {
    let mtime = std::fs::metadata(path).unwrap().modified().unwrap();
    let c = std::fs::read_to_string(path).unwrap();
    assert!(c.contains(from), "{} not in {}", from, path.display());
    std::fs::write(path, c.replace(from, to)).unwrap();
    std::fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(mtime)
        .unwrap();
}

#[test]
fn compile5() {
    let (root, cdir) = setup();
    let v = vars(vec![(
        "root",
        Value::Str(root.to_string_lossy().to_string()),
    )]);
    // Precondition (upstream: left behind by compile4.t): populate the cache.
    let pre = config(&root, cdir.to_str().unwrap());
    for n in ["foo", "complex", "divisionbyzero"] {
        let _ = pre.process_file(n, Some(v.clone()));
    }
    let blam_src = format!("{}/src/blam", root.display());
    let _ = pre.process_file(&blam_src, Some(v.clone()));

    // COMPILE_DIR with a trailing slash must be handled OK.
    let cdir_slash = format!("{}/", cdir.display());
    let cached = |n: &str| {
        PathBuf::from(
            format!("{}/{}/src/{}.ttc", cdir.display(), root.display(), n).replace("//", "/"),
        )
    };
    let (foo, bar, blam) = (cached("foo"), cached("complex"), cached("blam"));
    assert!(foo.is_file(), "cached foo");
    assert!(bar.is_file(), "cached bar");
    assert!(blam.is_file(), "cached blam");
    hack(&foo, "the foo file", "the newly hacked foo file");
    hack(&blam, "blam", "wam-bam");

    test_expect(
        &tt_test("compile5"),
        vec![("default", config(&root, &cdir_slash))],
        v,
    );
}
