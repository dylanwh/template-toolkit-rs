use template_toolkit::{value::Value, Template};

fn sub(v: &Value, k: &str) -> Value {
    match v {
        Value::Hash(h) => h.borrow().get(k).cloned().unwrap_or(Value::Undefined),
        _ => Value::Undefined,
    }
}

#[test]
fn trace_vars() {
    let tt = Template::builder().trace_vars(true).build().unwrap();
    let doc = tt
        .context()
        .compile_text(include_str!("corpus/trace_vars.tt"))
        .expect("compile");
    let vars = doc.variables_value();
    let used = |v: Value| v.is_defined();

    assert!(used(sub(&vars, "foo")), "foo is used");
    assert!(used(sub(&vars, "bar")), "bar is used");
    assert!(used(sub(&sub(&vars, "bar"), "baz")), "bar.baz is used");
    assert!(used(sub(&vars, "blam")), "blam is used");
    assert!(used(sub(&sub(&vars, "blam"), "0")), "blam.0 is used");
    assert!(used(sub(&vars, "wig")), "wig is used");
    assert!(used(sub(&sub(&vars, "wig"), "wam")), "wig.wam is used");
    assert!(
        used(sub(&sub(&sub(&vars, "wig"), "wam"), "bam")),
        "wig.wam.bam is used"
    );
    for letter in ["a", "b", "c", "d", "e"] {
        assert!(used(sub(&vars, letter)), "{} is used", letter);
    }
}
