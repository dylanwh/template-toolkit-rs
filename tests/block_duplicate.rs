use std::{cell::Cell, rc::Rc};

use template_toolkit::parser::Parser;

// Upstream goes through Template->process with a __WARN__ handler; the Rust
// Template builder does not expose the parser warn hook, so exercise the same
// parser code path directly.
#[test]
fn block_duplicate() {
    let seen = Rc::new(Cell::new(0));
    let s2 = seen.clone();
    let mut p = Parser::builder()
        .warn_handler(Rc::new(move |w: &str| {
            assert!(
                w.contains("Block redefined: b1"),
                "Unexpected warning: {}",
                w
            );
            s2.set(s2.get() + 1);
        }) as Rc<dyn Fn(&str)>)
        .build()
        .unwrap();
    p.parse(
        "[% BLOCK b1 %]first[% END %]\n[% BLOCK b1 %]second[% END %]\n",
        None,
    )
    .unwrap();
    assert_eq!(seen.get(), 1, "warning seen");
}
