mod common;
use common::*;
use template_toolkit::Template;

// Upstream skips the whole file when the Perl CGI module is missing.  The Rust
// CGI plugin provides query-string parsing, param() and params(); the
// checkbox_group case relies on CGI.pm's HTML generation (its expectation is
// computed by CGI.pm itself) and is skipped.
#[test]
fn cgi() {
    let corpus = include_str!("corpus/cgi.tt");
    let skip_from = corpus
        .find("-- test --\n[% FOREACH x = global.cgi.checkbox_group")
        .unwrap();
    let skip_to = skip_from + 1 + corpus[skip_from + 1..].find("-- test --").unwrap();
    let text = format!("{}{}", &corpus[..skip_from], &corpus[skip_to..]);
    test_expect(&text, vec![("default", Template::new())], vars(vec![]));
}
