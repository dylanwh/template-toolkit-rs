mod common;
use common::*;
use template_toolkit::{provider::IncludePathItem, Template};

#[test]
fn macro_() {
    let tt = Template::builder()
        .include_path(vec![IncludePathItem::Path(
            "corpora/Template-Toolkit-3.009/t/test/src".into(),
        )])
        .trim(true)
        .build()
        .unwrap();
    // Upstream sets EVAL_PERL => 1 for the final "perl macro" case
    // (MACRO ... PERL); Perl evaluation is out of scope, so that case is cut.
    let corpus = &tt_test("macro");
    let cut = corpus
        .find("-- test --\n-- name perl macro --")
        .expect("perl macro case");
    test_expect(&corpus[..cut], vec![("default", tt)], callsign());
}
