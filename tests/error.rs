use std::collections::HashMap;

use template_toolkit::Template;

#[test]
fn error() {
    let mut blocks = HashMap::new();
    blocks.insert("badinc".to_string(), "[% INCLUDE nosuchfile %]".into());
    let template = Template::builder().blocks(blocks).build().unwrap();
    assert!(template.process_file("badinc", None).is_err());
    let error = template.error().expect("error set");
    assert_eq!(error.type_(), "file");
    assert_eq!(error.info_str(), "nosuchfile: not found");
}
