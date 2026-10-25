// Upstream t/pod.t is skipped when Pod::POM is not installed.  The Pod plugin
// is a thin wrapper around the CPAN module Pod::POM, which has no Rust port,
// so this test is skipped (its cases are upstream's t/pod.t).
#[test]
#[ignore = "requires Pod::POM (Perl CPAN module); upstream skips when unavailable"]
fn pod() {}
