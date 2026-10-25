mod common;
use common::*;
use template_toolkit::{constants::CHOMP_ALL, value::Value, Template};

fn callsign() -> Vars {
    let w = [
        "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel", "india",
        "juliet", "kilo", "lima", "mike", "november", "oscar", "papa", "quebec", "romeo", "sierra",
        "tango", "umbrella", "victor", "whisky", "x-ray", "yankee", "zulu",
    ];
    ('a'..='z')
        .zip(w)
        .map(|(k, v)| (k.to_string(), Value::from(v)))
        .collect()
}

#[test]
fn case() {
    let def = Template::builder().post_chomp(CHOMP_ALL).build().unwrap();
    let any = Template::builder()
        .anycase(true)
        .post_chomp(CHOMP_ALL)
        .build()
        .unwrap();
    test_expect(
        &tt_test("case"),
        vec![("default", def), ("anycase", any)],
        callsign(),
    );
}
