mod common;
use common::*;
use template_toolkit::Template;

#[test]
fn outline() {
    let vanilla = Template::builder().build().unwrap();
    let outline = Template::builder()
        .tag_style("outline".to_string())
        .build()
        .unwrap();
    let outtag = Template::builder()
        .outline_tag("%%".to_string())
        .build()
        .unwrap();
    // quotemeta '$ ' -- in Perl the value is a regex source; outline_tag is a regex source, so pass quotemeta(.$ .)
    let shell = Template::builder()
        .outline_tag(r"\$\ ".to_string())
        .build()
        .unwrap();
    test_expect(
        &tt_test("outline"),
        vec![
            ("default", vanilla),
            ("outline", outline),
            ("outtag", outtag),
            ("shell", shell),
        ],
        callsign(),
    );
}
