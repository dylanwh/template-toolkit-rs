//! Template Toolkit in the browser: the Rust port compiled to WebAssembly,
//! for the playground page in `playground/www`.

use serde::Deserialize;
use template_toolkit::{
    constants::{DebugFlagsInput, CHOMP_COLLAPSE, CHOMP_GREEDY, CHOMP_NONE, CHOMP_ONE},
    schema::SchemaOptions,
    Template,
};
use wasm_bindgen::prelude::*;

/// Template options, as the page sends them (all optional).
#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct Options {
    tag_style: Option<String>,
    pre_chomp: Option<String>,
    post_chomp: Option<String>,
    trim: bool,
    interpolate: bool,
    anycase: bool,
    strict: bool,
    debug: Option<String>,
}

fn chomp(mode: Option<&str>) -> Result<usize, String> {
    match mode.unwrap_or("none") {
        "none" | "0" | "+" => Ok(CHOMP_NONE),
        "one" | "1" | "-" => Ok(CHOMP_ONE),
        "collapse" | "2" | "=" => Ok(CHOMP_COLLAPSE),
        "greedy" | "3" | "~" => Ok(CHOMP_GREEDY),
        other => Err(format!("unknown chomp mode: {other}")),
    }
}

fn build(options: JsValue) -> Result<Template, String> {
    let opts: Options = if options.is_undefined() || options.is_null() {
        Options::default()
    } else {
        serde_wasm_bindgen::from_value(options).map_err(|e| e.to_string())?
    };
    Template::builder()
        .maybe_tag_style(opts.tag_style.filter(|s| !s.is_empty()))
        .pre_chomp(chomp(opts.pre_chomp.as_deref())?)
        .post_chomp(chomp(opts.post_chomp.as_deref())?)
        .trim(opts.trim)
        .interpolate(opts.interpolate)
        .anycase(opts.anycase)
        .strict(opts.strict)
        .maybe_debug(
            opts.debug
                .filter(|s| !s.is_empty())
                .map(DebugFlagsInput::Text),
        )
        .build()
        .map_err(|e| e.to_string())
}

/// Process `template` and return its output; throws the Template Toolkit
/// error message (e.g. `parse error - input text line 2: ...`) on failure.
#[wasm_bindgen]
pub fn render(template: &str, options: JsValue) -> Result<String, String> {
    build(options)?
        .process_text(template, None)
        .map_err(|e| e.to_string())
}

/// Scan `template` without running it: JSON with `variables` (a JSON Schema
/// of its inputs), `modules` (USE), `filters` (non-standard ones) and
/// `warnings`.  Throws the parse error if it does not parse.
#[wasm_bindgen]
pub fn scan(template: &str, options: JsValue) -> Result<String, String> {
    let opts = SchemaOptions::default();
    let inferred = build(options)?
        .infer_schema_text(template, &opts)
        .map_err(|e| e.to_string())?;
    let mut report = inferred.report();
    report["warnings"] = inferred.warnings.into();
    Ok(report.to_string())
}

/// The Template Toolkit version this port follows.
#[wasm_bindgen]
pub fn version() -> String {
    "Template Toolkit 3.009 (Rust port)".to_string()
}
