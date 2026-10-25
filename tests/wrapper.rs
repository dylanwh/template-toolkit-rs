mod common;
use common::*;
#[allow(unused_imports)]
use template_toolkit::constants::*;
#[allow(unused_imports)]
use template_toolkit::provider::IncludePathItem;
#[allow(unused_imports)]
use template_toolkit::value::Value;
use template_toolkit::Template;

#[test]
fn wrapper() {
    let dir = "corpora/Template-Toolkit-3.009/t/test";
    let tt = Template::builder()
        .include_path(vec![
            IncludePathItem::Path(format!("{}/src", dir)),
            IncludePathItem::Path(format!("{}/lib", dir)),
        ])
        .trim(true)
        .build()
        .unwrap();
    test_expect(&tt_test("wrapper"), vec![("default", tt)], callsign());
}
