use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use template_toolkit::Template;

fn reasciify(s: &str) -> String {
    s.chars()
        .map(|c| {
            let ord = c as u32;
            if ord > 127 || (ord < 32 && ord != 10) {
                format!("\\x{{{:x}}}", ord)
            } else {
                c.to_string()
            }
        })
        .collect()
}

fn setup_tt(tempdir: &Path) -> Template {
    Template::builder()
        .absolute(true)
        .compile_dir(tempdir.to_string_lossy().to_string())
        .compile_ext(".ttcache".to_string())
        .build()
        .unwrap()
}

fn test_it(name: &str, tt: &Template, filenames: &BTreeMap<&str, PathBuf>, expect: &str) {
    for (encoding, file) in filenames {
        let output = match tt.process_file(file.to_str().unwrap(), None) {
            Ok(o) => o,
            Err(e) => e.to_string(),
        };
        assert_eq!(
            reasciify(&output),
            reasciify(expect),
            "{} - {}",
            name,
            encoding
        );
    }
}

/// Latin-1 style byte string, as in the Perl "\x{ef}\x{bb}..." literals.
fn bytes(v: &[u8]) -> Vec<u8> {
    v.to_vec()
}

#[test]
fn unicode() {
    let moose = "m\u{f8}\u{f8}se\u{2026}";
    let encoded: Vec<(&str, Vec<u8>)> = vec![
        ("UTF-8", bytes(b"\xef\xbb\xbfm\xc3\xb8\xc3\xb8se\xe2\x80\xa6")),
        ("UTF-16BE", bytes(b"\xfe\xff\x00m\x00\xf8\x00\xf8\x00s\x00e\x20\x26")),
        ("UTF-16LE", bytes(b"\xff\xfem\x00\xf8\x00\xf8\x00s\x00e\x00\x26\x20")),
        ("UTF-32BE", bytes(b"\x00\x00\xfe\xff\x00\x00\x00m\x00\x00\x00\xf8\x00\x00\x00\xf8\x00\x00\x00s\x00\x00\x00e\x00\x00\x20\x26")),
        ("UTF-32LE", bytes(b"\xff\xfe\x00\x00m\x00\x00\x00\xf8\x00\x00\x00\xf8\x00\x00\x00s\x00\x00\x00e\x00\x00\x00\x26\x20\x00\x00")),
    ];

    let base = std::env::temp_dir().join(format!("tt-unicode-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    let mut filenames: BTreeMap<&str, PathBuf> = BTreeMap::new();
    for (enc, data) in &encoded {
        let d = base.join(enc);
        fs::create_dir_all(&d).unwrap();
        let f = d.join(enc);
        fs::write(&f, data).unwrap();
        filenames.insert(*enc, f);
    }
    let cache = base.join("cache");
    fs::create_dir_all(&cache).unwrap();

    let tt = setup_tt(&cache);
    test_it("first try", &tt, &filenames, moose);
    test_it("in memory", &tt, &filenames, moose);
    let tt = setup_tt(&cache);
    test_it("from cache", &tt, &filenames, moose);
    test_it("in cache, in memory", &tt, &filenames, moose);

    let _ = fs::remove_dir_all(&base);
}
