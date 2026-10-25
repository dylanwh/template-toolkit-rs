//! Wall-clock time.  `wasm32-unknown-unknown` has no clock without a
//! JavaScript host (`SystemTime::now` panics there), so it reports the
//! epoch; every other target, including WASI, uses the system clock.

use std::time::Duration;

/// Time since the Unix epoch.
pub(crate) fn since_epoch() -> Duration {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        Duration::ZERO
    }
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
    }
}

/// Seconds since the Unix epoch.
pub(crate) fn unix_secs() -> i64 {
    since_epoch().as_secs() as i64
}
