//! Cache of compiled regexes whose patterns are only known at runtime:
//! TAGS start/end tags, and patterns passed to vmethods, filters and
//! plugins (`match`, `replace`, `split`, ...).  Fixed patterns use
//! `LazyLock` statics instead.
//!
//! Compiling a regex costs far more than running it, and templates tend to
//! reuse the same few patterns, often inside loops.  Failed compilations are
//! cached too, since callers fall back to `fancy_regex` or literal matching.

use std::{
    collections::HashMap,
    hash::Hash,
    sync::{LazyLock, Mutex},
};

/// Bound on entries per cache; a full cache is cleared rather than evicted
/// piecemeal, which is plenty for the handful of patterns templates use.
const MAX_ENTRIES: usize = 512;

/// Builder flags for [`regex_with`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Flags {
    pub multi_line: bool,
    pub dot_matches_new_line: bool,
}

type Cache<K, V> = LazyLock<Mutex<HashMap<K, Option<V>>>>;

static PLAIN: Cache<(String, Flags), regex::Regex> = LazyLock::new(Default::default);
static FANCY: Cache<String, fancy_regex::Regex> = LazyLock::new(Default::default);

fn cached<K: Hash + Eq, V: Clone>(
    cache: &Cache<K, V>,
    key: K,
    build: impl FnOnce(&K) -> Option<V>,
) -> Option<V> {
    let mut map = cache.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(hit) = map.get(&key) {
        return hit.clone();
    }
    if map.len() >= MAX_ENTRIES {
        map.clear();
    }
    let value = build(&key);
    map.insert(key, value.clone());
    value
}

/// `regex::Regex::new(pattern)`, cached; `None` if the pattern is invalid.
pub fn regex(pattern: &str) -> Option<regex::Regex> {
    regex_with(pattern, Flags::default())
}

/// Like [`regex`], with builder flags.
pub fn regex_with(pattern: &str, flags: Flags) -> Option<regex::Regex> {
    cached(&PLAIN, (pattern.to_string(), flags), |(p, f)| {
        regex::RegexBuilder::new(p)
            .multi_line(f.multi_line)
            .dot_matches_new_line(f.dot_matches_new_line)
            .build()
            .ok()
    })
}

/// `fancy_regex::Regex::new(pattern)`, cached (for look-around and
/// backreferences, which `regex` rejects).
pub fn fancy(pattern: &str) -> Option<fancy_regex::Regex> {
    cached(&FANCY, pattern.to_string(), |p| {
        fancy_regex::Regex::new(p).ok()
    })
}
