mod common;
use std::{cell::Cell, rc::Rc};

use common::*;
use template_toolkit::{
    error::{ControlFlow, TemplateException},
    value::Value,
    Template,
};

fn arg(a: &[Value], i: usize) -> Value {
    a.get(i).cloned().unwrap_or(Value::Undefined)
}

#[test]
fn varsv1() {
    let days = [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ];
    let day = Rc::new(Cell::new(-1i64));
    let count = Rc::new(Cell::new(0i64));
    let (c_n, c_up, c_down, c_reset) = (count.clone(), count.clone(), count.clone(), count.clone());

    let params = vars(vec![
        ("a", "alpha".into()),
        ("b", "bravo".into()),
        ("c", "charlie".into()),
        ("d", "delta".into()),
        ("e", "echo".into()),
        (
            "f",
            hash(vec![
                ("g", "golf".into()),
                ("h", "hotel".into()),
                (
                    "i",
                    hash(vec![("j", "juliet".into()), ("k", "kilo".into())]),
                ),
            ]),
        ),
        ("l", "lima".into()),
        ("r", "romeo".into()),
        ("s", "sierra".into()),
        ("t", "tango".into()),
        ("w", "whisky".into()),
        ("n", Value::function(move |_, _| Ok(Value::Int(c_n.get())))),
        (
            "up",
            Value::function(move |_, _| {
                c_up.set(c_up.get() + 1);
                Ok(Value::Int(c_up.get()))
            }),
        ),
        (
            "down",
            Value::function(move |_, _| {
                c_down.set(c_down.get() - 1);
                Ok(Value::Int(c_down.get()))
            }),
        ),
        (
            "reset",
            Value::function(move |a, _| {
                let v = arg(a, 0);
                c_reset.set(if v.is_truthy() { v.to_int() } else { 0 });
                Ok(Value::Int(c_reset.get()))
            }),
        ),
        ("undef", Value::function(|_, _| Ok(Value::Undefined))),
        ("zero", Value::function(|_, _| Ok(Value::Int(0)))),
        ("one", Value::function(|_, _| Ok(Value::Str("one".into())))),
        (
            "halt",
            Value::function(|_, _| {
                Err(ControlFlow::Exception(TemplateException::new(
                    "stop", "stopped",
                )))
            }),
        ),
        (
            "join",
            Value::function(|a, _| {
                let sep = arg(a, 0).to_tt_string();
                Ok(Value::Str(
                    a.iter()
                        .skip(1)
                        .map(|v| v.to_tt_string())
                        .collect::<Vec<_>>()
                        .join(&sep),
                ))
            }),
        ),
        (
            "split",
            Value::function(|a, _| {
                let sep = arg(a, 0).to_tt_string();
                let s = arg(a, 1).to_tt_string();
                let mut parts: Vec<&str> = s.split(sep.as_str()).collect();
                while parts.last() == Some(&"") {
                    parts.pop();
                }
                Ok(strs(&parts))
            }),
        ),
        (
            "magic",
            hash(vec![
                ("chant", "Hocus Pocus".into()),
                (
                    "spell",
                    Value::function(|a, _| {
                        Ok(Value::Str(
                            a.iter()
                                .map(|v| v.to_tt_string())
                                .collect::<Vec<_>>()
                                .join(" and a bit of "),
                        ))
                    }),
                ),
            ]),
        ),
        (
            "day",
            hash(vec![
                (
                    "prev",
                    Value::function(|_, _| {
                        Ok(Value::Str("All my troubles seemed so far away...".into()))
                    }),
                ),
                (
                    "this",
                    Value::function(|a, _| {
                        let w = arg(a, 0);
                        let when = if w.is_truthy() {
                            w.to_tt_string()
                        } else {
                            "Now".into()
                        };
                        Ok(Value::Str(format!(
                            "{} it looks as though they're here to stay.",
                            when
                        )))
                    }),
                ),
                (
                    "next",
                    Value::function(move |a, _| {
                        let mut dayno = arg(a, 0);
                        if !dayno.is_defined() {
                            day.set((day.get() + 1) % 7);
                            dayno = Value::Int(day.get());
                        }
                        Ok(Value::Str(days[dayno.to_int() as usize % 7].to_string()))
                    }),
                ),
            ]),
        ),
        (
            "belief",
            Value::function(|a, _| {
                let mut b = a
                    .iter()
                    .map(|v| v.to_tt_string())
                    .collect::<Vec<_>>()
                    .join(" and ");
                if b.is_empty() {
                    b = "<nothing>".into();
                }
                Ok(Value::Str(format!("Oh I believe in {}.", b)))
            }),
        ),
        (
            "people",
            Value::function(|_, _| Ok(strs(&["Tom", "Dick", "Larry"]))),
        ),
        ("letteralpha", "'alpha'".into()),
    ]);
    let procs = vec![
        (
            "default",
            Template::builder()
                .interpolate(true)
                .anycase(true)
                .v1dollar(true)
                .build()
                .unwrap(),
        ),
        (
            "notcase",
            Template::builder()
                .interpolate(true)
                .v1dollar(false)
                .build()
                .unwrap(),
        ),
    ];
    test_expect(&tt_test("varsv1"), procs, params);
}
