mod common;
use common::*;
use template_toolkit::Template;

#[test]
fn directry() {
    let cwd = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let dir = "corpora/Template-Toolkit-3.009/t/test/dir";
    let dot = dir.split('/').map(|_| "..").collect::<Vec<_>>().join("/");
    let params = vars(vec![
        ("cwd", cwd.into()),
        ("dir", dir.into()),
        ("dot", dot.into()),
    ]);
    let tt = Template::builder().build().unwrap();
    test_expect(&tt_test("directry"), vec![("default", tt)], params);
}
