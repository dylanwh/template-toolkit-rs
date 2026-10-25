use std::collections::HashMap;

use template_toolkit::{value::Value, Template};

#[test]
fn smoke() {
    let t = Template::new();
    let mut vars = HashMap::new();
    vars.insert("name".to_string(), Value::Str("World".into()));
    let out = t.process_text(
        "Hello [% name %]! [% FOREACH i IN [1,2,3] %][% i * 2 %],[% END %]",
        Some(vars),
    );
    println!("{:?}", out);
    assert_eq!(out.unwrap(), "Hello World! 2,4,6,");
}
