mod common;
use common::*;
use template_toolkit::{error::ControlFlow, value::Value, Template};

/// Port of the `commify` scalar op from t/vmethods/text.t.
fn commify(v: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    let mut s = v.to_tt_string();
    let c = args
        .first()
        .filter(|v| v.is_truthy())
        .map(|v| v.to_tt_string())
        .unwrap_or_else(|| ",".into());
    let n = args
        .get(1)
        .filter(|v| v.is_truthy())
        .map(|v| v.to_int())
        .unwrap_or(3);
    if n < 1 {
        return Ok(Value::Str(s));
    }
    let n = n as usize;
    // 1 while s/^([-+]?\d+)(\d{$n})/$1$c$2/;
    loop {
        let b = s.as_bytes();
        let start = if !b.is_empty() && (b[0] == b'-' || b[0] == b'+') {
            1
        } else {
            0
        };
        let mut end = start;
        while end < b.len() && b[end].is_ascii_digit() {
            end += 1;
        }
        // need at least one digit for $1 plus n digits for $2, within the digit run
        if end - start < n + 1 {
            break;
        }
        // greedy \d+ backtracks so that $2 is the last n digits of the run
        let split = end - n;
        s = format!("{}{}{}", &s[..split], c, &s[split..]);
        // Perl's anchored pattern re-runs from the start on the new string; the
        // run of digits is now cut at the separator
    }
    Ok(Value::Str(s))
}

#[test]
fn vmethods_text() {
    let params = vars(vec![
        ("undef", Value::Undefined),
        ("zero", 0.into()),
        ("one", 1.into()),
        ("animal", "cat".into()),
        ("string", "The cat sat on the mat".into()),
        ("spaced", "  The dog sat on the log".into()),
        ("word", "bird".into()),
        ("WORD", "BIRD".into()),
        ("the_bird", "\n  The  bird\n  is  the  word  \n  ".into()),
        ("quotable", "Tim O'Reilly said \"Oh really?\"".into()),
        ("markup", "a < b > & c \"d\" 'e'".into()),
    ]);
    let tt = Template::new();
    tt.context().define_vmethod("scalar", "commify", commify);
    tt.context().define_vmethod("item", "commas", commify);
    test_expect(
        include_str!("corpus/vmethods_text.tt"),
        vec![("default", tt)],
        params,
    );
}
