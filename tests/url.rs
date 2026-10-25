mod common;
use common::*;
use template_toolkit::{
    plugins::{create_url_callable, set_url_joint},
    value::Value,
    Template,
};

fn sort_params(query: &str) -> String {
    let (base, args) = match query.split_once('?') {
        Some((b, a)) => (b, a),
        None => (query, ""),
    };
    let mut tab: std::collections::BTreeMap<String, String> = Default::default();
    for a in args.split("&amp;").filter(|s| !s.is_empty()) {
        let k = a.split('=').next().unwrap_or("").to_string();
        tab.insert(k, a.to_string());
    }
    let args = tab.values().cloned().collect::<Vec<_>>().join("&amp;");
    if base.is_empty() {
        args
    } else {
        format!("{}?{}", base, args)
    }
}

#[test]
fn url() {
    let mk = |base: &str, args: Vec<(&str, Value)>| {
        create_url_callable(
            Some(base.to_string()),
            args.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        )
    };
    let urls = hash(vec![(
        "product",
        hash(vec![
            ("view", mk("/product", vec![])),
            ("add", mk("/product", vec![("action", "add".into())])),
            (
                "edit",
                mk(
                    "/product",
                    vec![("action", "edit".into()), ("style", "editor".into())],
                ),
            ),
        ]),
    )]);
    let params = vars(vec![
        ("url", urls),
        (
            "sorted",
            Value::function(|a, _| {
                Ok(Value::Str(sort_params(
                    &a.first().map(|v| v.to_tt_string()).unwrap_or_default(),
                )))
            }),
        ),
        (
            "no_escape",
            Value::function(|_, _| {
                set_url_joint("&");
                Ok(Value::Undefined)
            }),
        ),
    ]);
    let tt = Template::builder().interpolate(true).build().unwrap();
    test_expect(include_str!("corpus/url.tt"), vec![("default", tt)], params);
}
