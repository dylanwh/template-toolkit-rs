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
fn tags() {
    let params = vars(vec![
        ("a", "alpha".into()),
        ("b", "bravo".into()),
        ("c", "charlie".into()),
        ("d", "delta".into()),
        ("e", "echo".into()),
        ("tags", "my tags".into()),
        ("flags", "my flags".into()),
    ]);
    let procs = vec![
        (
            "basic",
            Template::builder().interpolate(true).build().unwrap(),
        ),
        (
            "htags",
            Template::builder()
                .tag_style("html".to_string())
                .build()
                .unwrap(),
        ),
        (
            "stags",
            Template::builder()
                .start_tag(r"\[\*".to_string())
                .end_tag(r"\*\]".to_string())
                .build()
                .unwrap(),
        ),
    ];
    test_expect(&tt_test("tags"), procs, params);
}
