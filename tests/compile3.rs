mod common;
use std::path::{Path, PathBuf};

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
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("compile3_src");
    copy_dir(Path::new("Template-Toolkit-3.009/t/test/src"), &dir);
    dir
}

use std::time::{Duration, SystemTime};

fn bump(path: &Path, secs: u64) {
    let t = SystemTime::now() + Duration::from_secs(secs);
    std::fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(t)
        .unwrap();
}

#[test]
fn compile3() {
    let dir = setup();
    let cfg = || {
        Template::builder()
            .post_chomp(1)
            .include_path(vec![IncludePathItem::Path(
                dir.to_string_lossy().to_string(),
            )])
            .compile_ext(".ttc".to_string())
    };
    let tt = cfg().build().unwrap();
    let err = tt
        .process_file("evalperl", None)
        .expect_err("evalperl without EVAL_PERL must fail");
    assert_eq!(err.type_, "perl");
    assert_eq!(err.info.to_tt_string(), "EVAL_PERL not set");

    // Upstream next rebuilds with EVAL_PERL => 1 and processes evalperl, which
    // needs a Perl interpreter (out of scope).  Compile `complex` instead so the
    // .ttc exists for the staleness checks below.
    let tt = cfg().build().unwrap();
    tt.process_file("complex", None).unwrap();
    let file = dir.join("complex");
    let ttc = dir.join("complex.ttc");
    assert!(ttc.is_file());

    let append = {
        let file = file.clone();
        move |s: &str| {
            use std::io::Write;
            std::thread::sleep(Duration::from_secs(2)); // let STAT_TTL expire, as upstream
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&file)
                .unwrap();
            f.write_all(s.as_bytes()).unwrap();
            drop(f);
            bump(&file, 10);
        }
    };
    let ttc_mtime = std::fs::metadata(&ttc).unwrap().modified().unwrap();
    append("\n");
    let bust = append.clone();
    let v = vars(vec![
        (
            "bust_it",
            Value::function(move |_, _| {
                bust("[% TRY %]");
                Ok(Value::Undefined)
            }),
        ),
        (
            "near_line",
            Value::function(|a, _| {
                let w = a[0].to_tt_string();
                let n = a[1].to_int();
                if let Some(i) = w.find("line ") {
                    let digits: String = w[i + 5..]
                        .chars()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    if let Ok(l) = digits.parse::<i64>() {
                        let out = format!("{}line {}ish{}", &w[..i], n, &w[i + 5 + digits.len()..]);
                        let diff = (l - n).abs();
                        return Ok(Value::Str(if diff < 4 {
                            out
                        } else {
                            format!("{} (where 'ish' means {}!)", out, diff)
                        }));
                    }
                }
                Ok(Value::Str("no idea what line number that is\n".into()))
            }),
        ),
    ]);
    let tt = cfg().build().unwrap();
    test_expect(include_str!("corpus/compile3.tt"), vec![("default", tt)], v);
    assert!(std::fs::metadata(&file).unwrap().modified().unwrap() > ttc_mtime);
}
