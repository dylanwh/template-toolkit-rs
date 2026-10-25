//! serde support: Rust data into templates and back.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use template_toolkit::{
    serde_value::{from_value, to_value, to_vars, Serde},
    value::Value,
    Template,
};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
enum Status {
    Open,
    Closed { reason: String },
    Moved(u32),
    Pair(i64, String),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct User {
    name: String,
    admin: bool,
    email: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
struct Page {
    title: String,
    users: Vec<User>,
    counts: BTreeMap<u32, u64>,
    status: Vec<Status>,
    ratio: f64,
    initial: char,
    nothing: (),
    big: u64,
}

fn page() -> Page {
    Page {
        title: "Report".into(),
        users: vec![
            User {
                name: "Ada".into(),
                admin: true,
                email: Some("ada@example.org".into()),
            },
            User {
                name: "Bob".into(),
                admin: false,
                email: None,
            },
        ],
        counts: [(1, 10), (2, 20)].into_iter().collect(),
        status: vec![
            Status::Open,
            Status::Closed {
                reason: "done".into(),
            },
            Status::Moved(42),
            Status::Pair(7, "x".into()),
        ],
        ratio: 0.5,
        initial: 'R',
        nothing: (),
        big: u64::MAX,
    }
}

fn render(tt: &Template, src: &str, vars: HashMap<String, Value>) -> String {
    tt.process_text(src, Some(vars))
        .unwrap_or_else(|e| panic!("{src}: {e}"))
}

#[test]
fn struct_as_template_variables() {
    let tt = Template::new();
    let vars = to_vars(&page()).unwrap();
    let out = render(
        &tt,
        "[% title %]: [% FOREACH u IN users %][% u.name %][% ' (admin)' IF u.admin %] \
         <[% u.email or 'none' %]>[% ', ' UNLESS loop.last %][% END %]",
        vars.clone(),
    );
    assert_eq!(out, "Report: Ada (admin) <ada@example.org>, Bob <none>");
    // Map keys are strings; numbers are stringified.
    assert_eq!(
        render(&tt, "[% counts.1 %]+[% counts.2 %]", vars.clone()),
        "10+20"
    );
    // Scalars: f64, char, unit (undefined), u64 beyond i64 (exact, as a string).
    assert_eq!(
        render(&tt, "[% ratio %] [% initial %] [[% nothing %]] [% nothing.defined ? 'def' : 'undef' %] [% big %]", vars.clone()),
        "0.5 R [] undef 18446744073709551615"
    );
    // Enums: unit variants are their name, others a one-key hash.
    assert_eq!(
        render(&tt, "[% status.0 %] [% status.1.Closed.reason %] [% status.2.Moved %] [% status.3.Pair.join('/') %]", vars),
        "Open done 42 7/x"
    );
}

#[test]
fn value_shapes() {
    assert!(matches!(to_value(&true).unwrap(), Value::Int(1)));
    assert_eq!(to_value(&false).unwrap().to_tt_string(), "");
    assert!(!to_value(&None::<i32>).unwrap().is_defined());
    assert!(matches!(to_value(&vec![1, 2]).unwrap(), Value::List(_)));
    assert!(matches!(to_value(&(1, "a")).unwrap(), Value::List(_)));
    let err = to_value(&HashMap::from([(vec![1], 2)])).unwrap_err();
    assert!(err.to_string().contains("map keys must be"), "{err}");
    let err = to_vars(&vec![1, 2]).unwrap_err();
    assert_eq!(
        err.to_string(),
        "template variables must be a struct or map, got ARRAY"
    );
}

#[test]
fn round_trip() {
    let users = page().users;
    let value = to_value(&users).unwrap();
    assert_eq!(from_value::<Vec<User>>(&value).unwrap(), users);
    let status = page().status;
    assert_eq!(
        from_value::<Vec<Status>>(&to_value(&status).unwrap()).unwrap(),
        status
    );
}

#[derive(Deserialize, Debug, PartialEq)]
struct Opts {
    width: usize,
    fill: Option<char>,
    upper: bool,
    tags: Vec<String>,
}

#[test]
fn deserializing_template_data_coerces_like_perl() {
    let tt = Template::new();
    // Data built by a template, as a typed callback receives it.
    tt.define_function("show", |opts: Serde<Opts>| format!("{:?}", opts.0));
    let out = tt
        .process_text(
            "[% show({ width => '8', upper => 'yes', tags => 'solo' }) %]",
            None,
        )
        .unwrap();
    assert_eq!(
        out,
        r#"Opts { width: 8, fill: None, upper: true, tags: ["solo"] }"#
    );
    let out = tt
        .process_text(
            "[% show({ width => 3, fill => '.', upper => 0, tags => ['a', 'b'] }) %]",
            None,
        )
        .unwrap();
    assert_eq!(
        out,
        r#"Opts { width: 3, fill: Some('.'), upper: false, tags: ["a", "b"] }"#
    );
    // Errors carry the argument position and serde's message.
    let err = tt
        .process_text("[% show({ fill => '.' }) %]", None)
        .unwrap_err()
        .to_string();
    assert_eq!(
        err,
        "undef error - bad argument #1 to `show`: missing field `width`"
    );
    let err = tt
        .process_text("[% show([1]) %]", None)
        .unwrap_err()
        .to_string();
    assert_eq!(
        err,
        "undef error - bad argument #1 to `show`: expected a hash, got ARRAY"
    );
    let err = tt
        .process_text("[% show({ width => -1, upper => 1, tags => [] }) %]", None)
        .unwrap_err()
        .to_string();
    assert_eq!(
        err,
        "undef error - bad argument #1 to `show`: -1 is out of range for u64"
    );
}

#[test]
fn enums_from_template_data() {
    let open: Status = from_value(&Value::from("Open")).unwrap();
    assert_eq!(open, Status::Open);
    let tt = Template::new();
    tt.define_function("status", |s: Serde<Status>| format!("{:?}", s.0));
    assert_eq!(
        tt.process_text(
            "[% status({ Closed => { reason => 'dup' } }) %] [% status({ Moved => '9' }) %]",
            None
        )
        .unwrap(),
        r#"Closed { reason: "dup" } Moved(9)"#
    );
}

#[test]
fn serde_return_values() {
    let tt = Template::new();
    tt.define_function("lookup", |name: String| {
        Serde(User {
            email: Some(format!("{}@example.org", name.to_lowercase())),
            name,
            admin: false,
        })
    });
    assert_eq!(
        tt.process_text("[% u = lookup('Cy'); u.name %] <[% u.email %]>", None)
            .unwrap(),
        "Cy <cy@example.org>"
    );
}

#[test]
fn value_to_and_from_json() {
    let value = to_value(&page().users).unwrap();
    let json = serde_json::to_string(&value).unwrap();
    // Hash keys are sorted for deterministic output; bools are Perl's 1 / "".
    assert_eq!(
        json,
        r#"[{"admin":1,"email":"ada@example.org","name":"Ada"},{"admin":"","email":null,"name":"Bob"}]"#
    );
    let back: Value = serde_json::from_str(r#"{"n": 1, "xs": [true, null, 2.5, "s"]}"#).unwrap();
    let tt = Template::new();
    let mut vars = HashMap::new();
    vars.insert("data".to_string(), back);
    assert_eq!(
        render(
            &tt,
            "[% data.n %] [% data.xs.size %] [% data.xs.0 %] [% data.xs.2 %]",
            vars
        ),
        "1 4 1 2.5"
    );
}
