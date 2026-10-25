mod common;
use common::*;
use template_toolkit::{
    constants::{CHOMP_ALL, STATUS_DONE},
    iterator::TemplateIterator,
    value::Value,
    Template,
};

fn list_len(v: &Value) -> usize {
    match v {
        Value::List(l) => l.borrow().len(),
        _ => panic!("not a list: {:?}", v),
    }
}

fn s(v: &Value) -> String {
    v.to_tt_string()
}

#[test]
fn iterator_api() {
    let data = strs(&["foo", "bar", "baz", "qux", "wiz", "woz", "waz"]);
    let i1 = TemplateIterator::new(&data);
    assert_eq!(s(&i1.get_first().0), "foo");
    assert_eq!(s(&i1.get_next().0), "bar");
    assert_eq!(s(&i1.get_next().0), "baz");

    let (rest, _) = i1.get_all();
    assert_eq!(list_len(&rest), 4);
    if let Value::List(l) = &rest {
        assert_eq!(s(&l.borrow()[0]), "qux");
        assert_eq!(s(&l.borrow()[3]), "waz");
    }

    let (val, err) = i1.get_next();
    assert!(!val.is_truthy());
    assert_eq!(err, Some(STATUS_DONE));

    let (val, err) = i1.get_all();
    assert!(!val.is_truthy());
    assert_eq!(err, Some(STATUS_DONE));

    let _ = i1.get_first();
    assert_eq!(s(&i1.get_first().0), "foo");
    assert_eq!(s(&i1.get_next().0), "bar");
    let (rest, _) = i1.get_all();
    assert_eq!(list_len(&rest), 5);

    let i2 = TemplateIterator::new(&data);
    let (rest, err) = i2.get_all();
    assert_eq!(list_len(&rest), 7);
    assert_eq!(err, None);
    let (val, err) = i2.get_all();
    assert!(!val.is_truthy());
    assert_eq!(err, Some(STATUS_DONE));

    let i3 = TemplateIterator::new(&strs(&["foo"]));
    let (rest, err) = i3.get_all();
    assert_eq!(list_len(&rest), 1);
    if let Value::List(l) = &rest {
        assert_eq!(s(&l.borrow()[0]), "foo");
    }
    assert_eq!(err, None);
    let (val, err) = i3.get_all();
    assert!(!val.is_truthy());
    assert_eq!(err, Some(STATUS_DONE));

    let i4 = TemplateIterator::new(&strs(&[]));
    let (val, err) = i4.get_all();
    assert!(!val.is_truthy());
    assert_eq!(err, Some(STATUS_DONE));
}

#[test]
fn iterator() {
    let params = vars(vec![(
        "data",
        strs(&["foo", "bar", "baz", "qux", "wiz", "woz", "waz"]),
    )]);
    let tt = Template::builder().post_chomp(CHOMP_ALL).build().unwrap();
    test_expect(&tt_test("iterator"), vec![("default", tt)], params);
}
