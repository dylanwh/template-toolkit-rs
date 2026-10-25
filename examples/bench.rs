//! The Rust side of the benchmarks against Perl's Template Toolkit;
//! bench/bench.pl is the Perl side, and scripts/bench.sh runs both.
//!
//!     cargo run --release --example bench -- MODE DATA DIR NAME...
//!
//! MODE is `output` (each template's output, to check the two sides
//! agree), `render` (the compiled template cached, as in a long-running
//! process) or `compile` (text to a runnable template, uncached).  Prints a
//! JSON line per template, measured as bench.pl measures.

use std::{collections::HashMap, time::Instant};

use serde_json::json;
use template_toolkit::{provider::IncludePathItem, value::Value, Template};

/// Per-operation time: calibrated so a sample takes at least 50 ms, then
/// 11 samples; the median and the best.
fn measure(mut op: impl FnMut()) -> serde_json::Value {
    for _ in 0..3 {
        op();
    }
    let mut n = 1u32;
    loop {
        let t = Instant::now();
        (0..n).for_each(|_| op());
        if t.elapsed().as_secs_f64() >= 0.05 {
            break;
        }
        n *= 2;
    }
    let mut per_op: Vec<f64> = (0..11)
        .map(|_| {
            let t = Instant::now();
            (0..n).for_each(|_| op());
            t.elapsed().as_secs_f64() / f64::from(n)
        })
        .collect();
    per_op.sort_by(f64::total_cmp);
    json!({ "median_us": per_op[5] * 1e6, "min_us": per_op[0] * 1e6, "iterations": n })
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [mode, data, dir, names @ ..] = args.as_slice() else {
        panic!("usage: bench MODE DATA DIR NAME...");
    };
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(data).unwrap()).unwrap();
    let vars: HashMap<String, Value> = template_toolkit::serde_value::to_vars(&json).unwrap();
    let tt = Template::builder()
        .include_path(vec![IncludePathItem::Path(dir.clone())])
        .build()
        .unwrap();

    for name in names {
        let file = format!("{name}.tt");
        let result = match mode.as_str() {
            "output" => json!({ "output": tt.process_file(&file, Some(vars.clone())).unwrap() }),
            "render" => measure(|| {
                std::hint::black_box(tt.process_file(&file, Some(vars.clone())).unwrap());
            }),
            "compile" => {
                let text = std::fs::read_to_string(format!("{dir}/{file}")).unwrap();
                measure(|| {
                    std::hint::black_box(tt.context().compile_text(&text).unwrap());
                })
            }
            other => panic!("unknown mode: {other}"),
        };
        let mut line = json!({ "name": name });
        line.as_object_mut()
            .unwrap()
            .extend(result.as_object().unwrap().clone());
        println!("{line}");
    }
}
