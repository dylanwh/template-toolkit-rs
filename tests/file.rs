mod common;
use std::os::unix::fs::MetadataExt;

use common::*;
use template_toolkit::Template;

#[test]
fn file() {
    let dir = "corpora/Template-Toolkit-3.009/t/test";
    let file = format!("{}/src/foo", dir);
    let m = std::fs::metadata(&file).unwrap();
    let mut v = vec![
        ("dir", dir.into()),
        ("file", file.as_str().into()),
        ("dev", m.dev().to_string().into()),
        ("ino", m.ino().to_string().into()),
        ("mode", m.mode().to_string().into()),
        ("nlink", m.nlink().to_string().into()),
        ("uid", m.uid().to_string().into()),
        ("gid", m.gid().to_string().into()),
        ("rdev", m.rdev().to_string().into()),
        ("size", m.size().to_string().into()),
        ("atime", m.atime().to_string().into()),
        ("mtime", m.mtime().to_string().into()),
        ("ctime", m.ctime().to_string().into()),
        ("blksize", m.blksize().to_string().into()),
        ("blocks", m.blocks().to_string().into()),
    ];
    let _ = &mut v;
    test_expect(
        &tt_test("file"),
        vec![("default", Template::new())],
        vars(v),
    );
}
