// Upstream t/zz-plugin-cycle.t is a developer-only Test::LeakTrace check of Perl
// reference cycles; it is skipped unless AUTOMATED_TESTING/RELEASE_TESTING is set
// and has no Rust equivalent (Rust has no GC cycles to trace this way).
#[test]
#[ignore = "perl-only leak trace test"]
fn zz_plugin_cycle() {}
