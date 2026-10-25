mod common;
use common::*;
use template_toolkit::{value::Value, Template};

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
fn anycase() {
    let vanilla = Template::builder().build().unwrap();
    let anycase = Template::builder()
        .anycase(true)
        .tag_style("outline".to_string())
        .build()
        .unwrap();
    test_expect(
        include_str!("corpus/anycase.tt"),
        vec![("default", vanilla), ("anycase", anycase)],
        callsign(),
    );
}
