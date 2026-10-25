mod common;
use common::*;
use template_toolkit::{constants::DebugFlagsInput, value::Value, Template};

#[test]
fn foreach() {
    let (a, b, c, d, l, o, r, u, w) = (
        "alpha", "bravo", "charlie", "delta", "lima", "oscar", "romeo", "uncle", "whisky",
    );
    let days = [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ];
    let months = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    let person = |id: &str, name: &str| hash(vec![("id", id.into()), ("name", name.into())]);
    let person2 = |id: &str, code: &str, name: &str| {
        hash(vec![
            ("id", id.into()),
            ("code", code.into()),
            ("name", name.into()),
        ])
    };
    let months_v = strs(&months);
    let params = vars(vec![
        ("a", a.into()),
        ("b", b.into()),
        ("c", c.into()),
        ("C", c.to_uppercase().into()),
        ("d", d.into()),
        ("l", l.into()),
        ("o", o.into()),
        ("r", r.into()),
        ("u", u.into()),
        ("w", w.into()),
        ("seta", strs(&[a, b, w])),
        ("setb", strs(&[c, l, o, u, d])),
        (
            "users",
            Value::list(vec![
                person("abw", "Andy Wardley"),
                person("sam", "Simon Matthews"),
            ]),
        ),
        ("item", "foo".into()),
        ("items", strs(&["foo", "bar"])),
        ("days", strs(&days)),
        ("months", Value::function(move |_, _| Ok(months_v.clone()))),
        (
            "format",
            Value::function(|args, _| {
                let f = args
                    .first()
                    .map(|v| v.to_tt_string())
                    .unwrap_or_else(|| "%s".into());
                Ok(Value::function(move |a, _| {
                    Ok(Value::Str(f.replacen(
                        "%s",
                        &a.first().map(|v| v.to_tt_string()).unwrap_or_default(),
                        1,
                    )))
                }))
            }),
        ),
        (
            "people",
            Value::list(vec![
                person2("abw", "abw", "Andy Wardley"),
                person2("aaz", "zaz", "Azbaz Azbaz Zazbazzer"),
                person2("bcd", "dec", "Binary Coded Decimal"),
                person2("efg", "zzz", "Extra Fine Grass"),
            ]),
        ),
        (
            "sections",
            hash(vec![
                ("one", "Section One".into()),
                ("two", "Section Two".into()),
                ("three", "Section Three".into()),
                ("four", "Section Four".into()),
            ]),
        ),
        (
            "nested",
            Value::list(vec![strs(&["a", "b", "c"]), strs(&["x", "y", "z"])]),
        ),
    ]);

    let template = Template::builder()
        .interpolate(true)
        .post_chomp(template_toolkit::constants::CHOMP_ALL)
        .anycase(false)
        .build()
        .unwrap();
    let ttdebug = Template::builder()
        .debug(DebugFlagsInput::Number(1))
        .debug_format(String::new())
        .build()
        .unwrap();
    test_expect(
        include_str!("corpus/foreach.tt"),
        vec![("default", template), ("debug", ttdebug)],
        params,
    );
}
