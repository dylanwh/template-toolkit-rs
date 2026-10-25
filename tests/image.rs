mod common;
use common::*;
use template_toolkit::Template;

#[test]
fn image() {
    // Perl skips unless Image::Info/Image::Size exist; the Rust plugin is native.
    let dir = "Template-Toolkit-3.009/images";
    let params = vars(vec![
        ("dir", dir.into()),
        (
            "file",
            hash(vec![
                ("logo", format!("{dir}/ttdotorg.gif").as_str().into()),
                ("power", format!("{dir}/tt2power.gif").as_str().into()),
                ("lname", "ttdotorg.gif".into()),
            ]),
        ),
    ]);
    let tt = Template::builder().build().unwrap();
    test_expect(
        include_str!("corpus/image.tt"),
        vec![("default", tt)],
        params,
    );
}
