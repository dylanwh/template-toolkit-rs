mod common;
use common::*;
use template_toolkit::{value::Value, Template};

const FORMAT_DEFAULT: &str = "%H:%M:%S %d-%b-%Y";

fn strftime_local(fmt: &str, t: i64) -> String {
    use std::ffi::CString;
    unsafe {
        let tt: libc::time_t = t as libc::time_t;
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&tt, &mut tm);
        let cf = CString::new(fmt).unwrap();
        let mut buf = vec![0u8; 512];
        let n = libc::strftime(
            buf.as_mut_ptr() as *mut libc::c_char,
            buf.len(),
            cf.as_ptr(),
            &tm,
        );
        buf.truncate(n);
        String::from_utf8_lossy(&buf).into_owned()
    }
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[test]
fn date_offset() {
    // local $ENV{TZ} = 'GMT'
    std::env::set_var("TZ", "GMT");
    extern "C" {
        fn tzset();
    }
    unsafe {
        tzset();
    }
    // check_tz(): the DATA expects +0000 for local 2001-09-30 12:59
    assert_eq!(
        strftime_local("%z", 1001854740),
        "+0000",
        "TZ GMT not showing as +0000"
    );

    let fmt = hash(vec![
        ("default", FORMAT_DEFAULT.into()),
        ("time", "%H:%M:%S".into()),
        ("date", "%d-%b-%Y".into()),
        ("timeday", "the time is %H:%M:%S on %A".into()),
    ]);
    let params = vars(vec![
        ("time", Value::Int(now_secs())),
        ("format", fmt),
        (
            "now",
            Value::function(|a, _| {
                let f = a
                    .first()
                    .map(|v| v.to_tt_string())
                    .filter(|s| !s.is_empty())
                    .unwrap_or(FORMAT_DEFAULT.into());
                Ok(Value::Str(strftime_local(&f, now_secs())))
            }),
        ),
        ("date_calc", Value::Int(0)),
    ]);
    std::thread::sleep(std::time::Duration::from_secs(1));
    let tt = Template::builder()
        .post_chomp(template_toolkit::constants::CHOMP_ALL)
        .build()
        .unwrap();
    test_expect(
        include_str!("corpus/date_offset.tt"),
        vec![("default", tt)],
        params,
    );
}
