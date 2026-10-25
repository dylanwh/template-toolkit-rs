mod common;
use common::*;
use template_toolkit::Template;

// The first three upstream cases check Perl runtime warnings emitted by
// evaluated generated Perl code ("Argument \"\" isn't numeric", "(eval)" file
// names) and FILE_INFO line tracking in Perl; they are Perl-specific and are
// skipped.  Only the parse-error-with-line-number case is ported.
#[test]
fn fileline() {
    let dir = "Template-Toolkit-3.009/t/test/lib";
    let tt = Template::builder()
        .include_path(vec![template_toolkit::provider::IncludePathItem::Path(
            dir.to_string(),
        )])
        .build()
        .unwrap();
    let corpus = include_str!("corpus/fileline.tt");
    let i = corpus.rfind("-- test --").unwrap();
    test_expect(&corpus[i..], vec![("err", tt)], vars(vec![]));
}
