mod common;
use common::*;
use template_toolkit::Template;

// The upstream test installs a Perl subclass of Template::Directive via
// FACTORY to rewrite constant.* idents; custom Perl factory classes are
// out of scope, so only the first (plain) case is ported.
#[test]
fn factory() {
    let corpus = &tt_test("factory");
    let end = corpus.rfind("-- test --").unwrap();
    let v = vars(vec![(
        "foo",
        hash(vec![
            ("bar", "Place to purchase drinks".into()),
            ("baz", "Short form of \"Basil\"".into()),
        ]),
    )]);
    test_expect(&corpus[..end], vec![("default", Template::new())], v);
}
