mod common;
use common::*;
use template_toolkit::{value::Value, Template};

#[test]
fn html() {
    // Upstream sets `entities` true when HTML::Entities or Apache::Util is
    // available; the Rust HTML plugin/filters do not depend on either, so
    // pass 1 to exercise the entity cases.
    let params = vars(vec![("entities", Value::Int(1))]);
    let tt = Template::builder().build().unwrap();
    // corpus is ISO-8859-1 (as read byte-wise by Perl)
    let corpus: String = tt_test_bytes("html").iter().map(|&b| b as char).collect();
    test_expect(&corpus, vec![("default", tt)], params);
}
