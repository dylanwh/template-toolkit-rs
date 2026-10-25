mod common;
use common::*;
use template_toolkit::Template;

#[test]
fn stash_xs_unicode() {
    let params = vars(vec![
        ("ascii", "key".into()),
        ("utf8", "ключ".into()),
        (
            "hash",
            hash(vec![("key", "value".into()), ("ключ", "значение".into())]),
        ),
        ("str", "щука".into()),
    ]);
    test_expect(
        &tt_test("stash-xs-unicode"),
        vec![("default", Template::new())],
        params,
    );
}
