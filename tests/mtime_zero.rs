use template_toolkit::Template;

#[test]
fn mtime_zero() {
    let content = "hello, world\n";
    let path = std::env::temp_dir().join(format!("tt-mtime-zero-{}", std::process::id()));
    std::fs::write(&path, content).unwrap();
    let file = path.to_string_lossy().into_owned();

    let out = Template::builder()
        .absolute(true)
        .build()
        .unwrap()
        .process_file(&file, None)
        .expect("process tmpfile");
    assert_eq!(out, content);

    // utime 0, 0, $tmpfile
    let f = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    f.set_modified(std::time::UNIX_EPOCH).unwrap();
    drop(f);

    let out = Template::builder()
        .absolute(true)
        .build()
        .unwrap()
        .process_file(&file, None)
        .expect("process tmpfile [utime=0]");
    assert_eq!(out, content);
    let _ = std::fs::remove_file(&path);
}
