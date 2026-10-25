// t/base.t tests Template::Base, the Perl class-construction mixin
// (new/_init/error/module_version with $ERROR package variables).  It has no
// Rust counterpart: constructors are bon builders returning Result, so error
// propagation is covered by the type system.  Skipped.
#[test]
#[ignore = "Template::Base is a Perl OO mixin with no Rust equivalent"]
fn base() {}
