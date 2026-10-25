use std::{fs, path::Path};

use template_toolkit::Template;

fn remove_tree(p: &str) {
    let path = Path::new(p);
    if path.is_dir() {
        let _ = fs::remove_dir_all(path);
    } else if path.exists() {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn process_dir() {
    let testdir = "testdir";
    let cachedir = "ttcache";
    remove_tree(cachedir);
    remove_tree(testdir);

    let mk = || {
        Template::builder()
            .compile_dir(cachedir.to_string())
            .build()
            .unwrap()
    };
    let tt1 = mk();

    let data = "This is the first test\n[% TRY; PROCESS \"testdir\"; CATCH e; \"error: e\"; END; %]\nThis is the end.\n";
    let expected1 = format!("file error - {}: not found", testdir);
    let expected2 = format!("file error - ./{}: not a file", testdir);
    let expected3 = "This is the first test\n\nThis is the end.\n";

    assert!(tt1.process_text(data, None).is_err());
    assert_eq!(
        tt1.error().unwrap().to_string(),
        expected1,
        "Error on missing file"
    );

    fs::create_dir(cachedir).unwrap();
    assert!(Path::new(cachedir).is_dir(), "Made cache dir");
    fs::create_dir(testdir).unwrap();
    assert!(Path::new(testdir).is_dir(), "Made test dir");

    let tt2 = mk();
    assert!(tt2.process_text(data, None).is_err());
    assert_eq!(
        tt2.error().unwrap().to_string(),
        expected2,
        "Error on PROCESSing directory"
    );

    assert!(
        !Path::new(&format!("{}/{}", cachedir, testdir)).is_file(),
        "Erroneous creation of 0b file with name of folder in cache folder"
    );

    fs::remove_dir(testdir).unwrap();
    fs::write(testdir, "").unwrap();

    let tt3 = mk();
    let ret = tt3.process_text(data, None).expect("process");
    assert_eq!(ret, expected3, "Correctly PROCESSed file");

    remove_tree(cachedir);
    remove_tree(testdir);
}
