mod common;
use common::*;
use template_toolkit::{
    error::{ControlFlow, TemplateException},
    strftime,
    value::Value,
    Template,
};

const FORMAT_DEFAULT: &str = "%H:%M:%S %d-%b-%Y";

// Stand-ins for POSIX::strftime/mktime/localtime from the upstream test.
fn strftime_local(fmt: &str, t: i64) -> String {
    strftime::strftime(fmt, &strftime::from_epoch(t, false), None)
}

fn time_locale(t: i64, fmt: &str, locale: &str) -> String {
    strftime::strftime(fmt, &strftime::from_epoch(t, false), Some(locale))
}

fn date_locale(t: &str, fmt: &str, locale: &str) -> Result<String, TemplateException> {
    let parts: Vec<&str> = t.split(['/', ' ', ':', '-']).collect();
    if parts.len() < 6 {
        return Err(TemplateException::new(
            "date",
            format!("bad time/date string:  expects 'h:m:s d:m:y'  got: '{}'", t),
        ));
    }
    let n: Vec<i32> = parts.iter().map(|p| p.parse().unwrap_or(0)).collect();
    let tm = strftime::mktime_local(n[5], n[4], n[3], n[0], n[1], n[2]).unwrap();
    Ok(strftime::strftime(fmt, &tm, Some(locale)))
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[test]
fn date() {
    let time = now_secs();
    let fmt = hash(vec![
        ("default", FORMAT_DEFAULT.into()),
        ("time", "%H:%M:%S".into()),
        ("date", "%d-%b-%Y".into()),
        ("timeday", "the time is %H:%M:%S on %A".into()),
    ]);
    let params = vars(vec![
        ("time", Value::Int(time)),
        ("format", fmt),
        ("timestr", strftime_local("%H:%M:%S", time).into()),
        ("datestr", strftime_local("%d-%b-%Y", time).into()),
        (
            "daystr",
            strftime_local("the time is %H:%M:%S on %A", time).into(),
        ),
        ("defstr", strftime_local(FORMAT_DEFAULT, time).into()),
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
        (
            "time_locale",
            Value::function(|a, _| {
                let g = |i: usize| a.get(i).map(|v| v.to_tt_string()).unwrap_or_default();
                Ok(Value::Str(time_locale(
                    a.first().map(|v| v.to_number() as i64).unwrap_or(0),
                    &g(1),
                    &g(2),
                )))
            }),
        ),
        (
            "date_locale",
            Value::function(|a, _| {
                let g = |i: usize| a.get(i).map(|v| v.to_tt_string()).unwrap_or_default();
                date_locale(&g(0), &g(1), &g(2))
                    .map(Value::Str)
                    .map_err(ControlFlow::Exception)
            }),
        ),
        // Date::Calc is a Perl module; the Date::Calc-dependent case is skipped.
        ("date_calc", Value::Int(0)),
    ]);
    std::thread::sleep(std::time::Duration::from_secs(1));
    let tt = Template::builder()
        .post_chomp(template_toolkit::constants::CHOMP_ALL)
        .build()
        .unwrap();
    test_expect(&tt_test("date"), vec![("default", tt)], params);
}
