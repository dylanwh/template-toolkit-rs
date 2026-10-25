use template_toolkit::error::TemplateException;

#[test]
fn exception() {
    let e1 = TemplateException::new("e1.type", "e1.info");
    let e2 = TemplateException::with_text("e2.type", "e2.info", "the current output buffer");
    assert_eq!(e1.type_(), "e1.type");
    assert_eq!(e2.info().to_tt_string(), "e2.info");
    let (t, i) = e1.type_info();
    assert_eq!(t, "e1.type");
    assert_eq!(i, "e1.info");
    assert_eq!(e2.as_string(), "e2.type error - e2.info");
    assert_eq!(e2.text(), "the current output buffer");
    let mut e2 = e2;
    e2.prepend_text("text to prepend ");
    assert_eq!(e2.text(), "text to prepend the current output buffer");
    let handlers = ["something", "e2", "e1.type"];
    assert_eq!(e1.select_handler(&handlers), Some("e1.type"));
    assert_eq!(e2.select_handler(&handlers), Some("e2"));
    let e3 = TemplateException::new("e3.type", "e3.info");
    assert_eq!(e3.text(), "");
    assert_eq!(e3.as_string(), "e3.type error - e3.info");
    assert!(e3 != *"fish");
}
