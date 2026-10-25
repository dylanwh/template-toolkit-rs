// Upstream t/tiedhash.t exercises Perl `tie` magic (Tie::StdHash / Tie::StdArray
// subclasses whose FETCH/STORE add prefixes) through Template::Stash and
// Template::Stash::XS, and itself calls skip_all() unless the XS stash is in use.
// Perl tied variables have no Rust analogue (a Value::Hash is a plain
// Rc<RefCell<HashMap>> with no FETCH/STORE hooks), so this test is skipped.
#[test]
#[ignore = "perl tie semantics (tied hash/array FETCH/STORE) not applicable to Rust values"]
fn tiedhash() {}
