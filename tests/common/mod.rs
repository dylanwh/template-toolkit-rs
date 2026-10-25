//! Rust port of Template::Test::test_expect().
#![allow(dead_code)]

use std::collections::HashMap;

use template_toolkit::{value::Value, Template};

pub type Vars = HashMap<String, Value>;

/// The Perl TT distribution scripts/fetch-corpora.sh unpacks.  The ported
/// tests read their cases (t/*.t) and fixtures (t/test) from it, as they
/// aren't ours to ship.
pub const TT_DIST: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/corpora/Template-Toolkit-3.009"
);

/// The test cases of upstream `t/{name}.t`: everything after its
/// `__DATA__` or `__END__` line.
pub fn tt_test(name: &str) -> String {
    String::from_utf8(tt_test_bytes(name)).unwrap_or_else(|e| panic!("t/{name}.t: {e}"))
}

/// `tt_test` for the few that aren't UTF-8.
pub fn tt_test_bytes(name: &str) -> Vec<u8> {
    let path = format!("{TT_DIST}/t/{name}.t");
    let text = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("{path}: {e} (run scripts/fetch-corpora.sh to fetch Perl TT)"));
    let start = [&b"\n__DATA__\n"[..], b"\n__END__\n"]
        .iter()
        .filter_map(|marker| {
            text.windows(marker.len())
                .position(|w| w == *marker)
                .map(|i| i + marker.len())
        })
        .min()
        .unwrap_or_else(|| panic!("{path}: no __DATA__ or __END__ section"));
    text[start..].to_vec()
}

pub fn hash(pairs: Vec<(&str, Value)>) -> Value {
    Value::hash(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

pub fn strs(items: &[&str]) -> Value {
    Value::list(items.iter().map(|s| Value::Str(s.to_string())).collect())
}

pub fn vars(pairs: Vec<(&str, Value)>) -> Vars {
    pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

fn strip_trailing(s: &str) -> &str {
    s.trim_end_matches(['\n', '\r'])
}

/// Strip leading line matching `^\s*-- name --\n` style directives.
fn take_directive(input: &str, kw: &str) -> (Option<String>, String) {
    // Perl uses /^\s*-- kw ... --\s*\n/m: the first matching line anywhere is removed.
    let mut off = 0;
    for raw in input.split_inclusive('\n') {
        let end = off + raw.len();
        let line = raw.trim();
        let l = line.to_ascii_lowercase();
        if l.starts_with("--") && l.ends_with("--") && l.len() >= 4 {
            let inner = l[2..l.len() - 2].trim();
            if let Some(rest) = inner.strip_prefix(kw) {
                if rest.is_empty() || rest.starts_with(|c: char| c == ':' || c.is_whitespace()) {
                    let orig = line[2..line.len() - 2].trim();
                    let arg = orig[kw.len()..].trim_start_matches(':').trim().to_string();
                    return (Some(arg), format!("{}{}", &input[..off], &input[end..]));
                }
            }
        }
        off = end;
    }
    (None, input.to_string())
}

fn is_marker(line: &str, kw: &str) -> bool {
    let t = line.trim().to_ascii_lowercase();
    t.len() >= 4 && t.starts_with("--") && t.ends_with("--") && t[2..t.len() - 2].trim() == kw
}

fn split_on(input: &str, kw: &str) -> Vec<String> {
    let mut parts = vec![String::new()];
    for line in input.split_inclusive('\n') {
        if is_marker(line, kw) {
            parts.push(String::new());
        } else {
            parts.last_mut().unwrap().push_str(line);
        }
    }
    parts
}

/// Run a corpus of `-- test --` / `-- expect --` cases.  `procs` is a list of
/// named processors; the first is the default.  Panics listing all failures.
pub fn test_expect(corpus: &str, procs: Vec<(&str, Template)>, params: Vars) {
    let mut input: String = corpus
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| format!("{}\n", l))
        .collect();
    if let Some(i) = input.find("-- start --") {
        input = input[i + "-- start --".len()..].to_string();
    }
    if let Some(i) = input.find("-- stop --") {
        input.truncate(i);
    }
    let mut tests = split_on(&input, "test");
    if tests[0].trim().is_empty() {
        tests.remove(0);
    }

    let mut cur = 0usize;
    let mut failures = Vec::new();
    for (n, t) in tests.iter().enumerate() {
        let (name, t) = take_directive(t, "name");
        let name = name.unwrap_or_else(|| format!("template text {}", n + 1));
        let (_todo, t) = take_directive(&t, "todo");
        let mut parts = split_on(&t, "expect").into_iter();
        let mut src = parts.next().unwrap_or_default();
        let mut expect = parts.next().unwrap_or_default();
        let (usename, s2) = take_directive(&src, "use");
        if let Some(u) = usename {
            if let Some(i) = procs.iter().position(|(n, _)| *n == u) {
                cur = i;
            }
            src = s2;
        }
        let tt = &procs[cur].1;
        let output = match tt.process_text(&src, Some(params.clone())) {
            Ok(o) => o,
            Err(e) => {
                failures.push(format!(
                    "[{}] process FAILED: {}\n  input: {:?}",
                    name, e, src
                ));
                continue;
            }
        };
        let (pk, e2) = {
            let first = expect.lines().next().unwrap_or("");
            let t = first.trim();
            if t.starts_with("--")
                && t.trim_start_matches('-').trim_end_matches('-').trim() == "process"
                && !t.is_empty()
            {
                (
                    true,
                    expect[expect.find('\n').map(|i| i + 1).unwrap_or(expect.len())..].to_string(),
                )
            } else {
                (false, String::new())
            }
        };
        if pk {
            match tt.process_text(&e2, Some(params.clone())) {
                Ok(o) => expect = o,
                Err(e) => {
                    failures.push(format!("[{}] expect-process FAILED: {}", name, e));
                    continue;
                }
            }
        }
        let (e, o) = (strip_trailing(&expect), strip_trailing(&output));
        if e != o {
            failures.push(format!(
                "[{}] MATCH FAILED\n  input: {:?}\n expect: {:?}\n output: {:?}",
                name, src, e, o
            ));
        }
    }
    if !failures.is_empty() {
        panic!(
            "{}/{} cases failed:\n{}",
            failures.len(),
            tests.len(),
            failures.join("\n")
        );
    }
}

/// Template::Test::callsign()
pub fn callsign() -> Vars {
    let pairs: Vec<(&str, &str)> = vec![
        ("a", "alpha"),
        ("b", "bravo"),
        ("c", "charlie"),
        ("d", "delta"),
        ("e", "echo"),
        ("f", "foxtrot"),
        ("g", "golf"),
        ("h", "hotel"),
        ("i", "india"),
        ("j", "juliet"),
        ("k", "kilo"),
        ("l", "lima"),
        ("m", "mike"),
        ("n", "november"),
        ("o", "oscar"),
        ("p", "papa"),
        ("q", "quebec"),
        ("r", "romeo"),
        ("s", "sierra"),
        ("t", "tango"),
        ("u", "umbrella"),
        ("v", "victor"),
        ("w", "whisky"),
        ("x", "x-ray"),
        ("y", "yankee"),
        ("z", "zulu"),
    ];
    pairs
        .into_iter()
        .map(|(k, v)| (k.to_string(), Value::Str(v.to_string())))
        .collect()
}
