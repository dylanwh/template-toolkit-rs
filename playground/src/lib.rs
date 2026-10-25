//! Template Toolkit in the browser: the Rust port compiled to WebAssembly,
//! for the playground page in `playground/www`.

use serde::Deserialize;
use template_toolkit::{
    constants::{DebugFlagsInput, CHOMP_COLLAPSE, CHOMP_GREEDY, CHOMP_NONE, CHOMP_ONE},
    parser::{lexer::Span, parser_build_builder as parser_state, Parser, ParserBuildBuilder},
    schema::SchemaOptions,
    template_build_builder as template_state, Template, TemplateBuildBuilder,
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

impl Options {
    fn from_js(options: JsValue) -> Result<Self, String> {
        if options.is_undefined() || options.is_null() {
            Ok(Self::default())
        } else {
            serde_wasm_bindgen::from_value(options).map_err(|e| e.to_string())
        }
    }

    fn tag_style(&self) -> Option<String> {
        self.tag_style.clone().filter(|s| !s.is_empty())
    }
}

/// The page's options as a builder method (bon's custom methods, on
/// builders from another crate): `Template::builder().options(&opts)?`.
trait WithOptions {
    /// The builder with those options set.
    type Set;
    fn options(self, opts: &Options) -> Result<Self::Set, String>;
}

impl<S: template_state::State> WithOptions for TemplateBuildBuilder<S>
where
    S::TagStyle: template_state::IsUnset,
    S::PreChomp: template_state::IsUnset,
    S::PostChomp: template_state::IsUnset,
    S::Trim: template_state::IsUnset,
    S::Interpolate: template_state::IsUnset,
    S::Anycase: template_state::IsUnset,
    S::Strict: template_state::IsUnset,
    S::Debug: template_state::IsUnset,
{
    type Set = TemplateBuildBuilder<
        template_state::SetDebug<
            template_state::SetStrict<
                template_state::SetAnycase<
                    template_state::SetInterpolate<
                        template_state::SetTrim<
                            template_state::SetPostChomp<
                                template_state::SetPreChomp<template_state::SetTagStyle<S>>,
                            >,
                        >,
                    >,
                >,
            >,
        >,
    >;

    fn options(self, opts: &Options) -> Result<Self::Set, String> {
        Ok(self
            .maybe_tag_style(opts.tag_style())
            .pre_chomp(chomp(opts.pre_chomp.as_deref())?)
            .post_chomp(chomp(opts.post_chomp.as_deref())?)
            .trim(opts.trim)
            .interpolate(opts.interpolate)
            .anycase(opts.anycase)
            .strict(opts.strict)
            .maybe_debug(
                opts.debug
                    .clone()
                    .filter(|s| !s.is_empty())
                    .map(DebugFlagsInput::Text),
            ))
    }
}

/// The options that change how a template parses.
impl<S: parser_state::State> WithOptions for ParserBuildBuilder<S>
where
    S::TagStyle: parser_state::IsUnset,
    S::PreChomp: parser_state::IsUnset,
    S::PostChomp: parser_state::IsUnset,
    S::Interpolate: parser_state::IsUnset,
    S::Anycase: parser_state::IsUnset,
{
    type Set = ParserBuildBuilder<
        parser_state::SetAnycase<
            parser_state::SetInterpolate<
                parser_state::SetPostChomp<parser_state::SetPreChomp<parser_state::SetTagStyle<S>>>,
            >,
        >,
    >;

    fn options(self, opts: &Options) -> Result<Self::Set, String> {
        Ok(self
            .maybe_tag_style(opts.tag_style())
            .pre_chomp(chomp(opts.pre_chomp.as_deref())?)
            .post_chomp(chomp(opts.post_chomp.as_deref())?)
            .interpolate(opts.interpolate)
            .anycase(opts.anycase))
    }
}

fn build(options: JsValue) -> Result<Template, String> {
    Template::builder()
        .options(&Options::from_js(options)?)?
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

/// Highlight and check `template` as the parser sees it, without running
/// it: JSON with `tokens`, `[start, end, kind]` triples for the editor
/// (kinds as in `template_toolkit::parser::highlight::Lexeme`), and
/// `error`, null or `{message, start, end}` where parsing stops.  Offsets
/// are UTF-16 code units, as JavaScript strings count them.
#[wasm_bindgen]
pub fn analyze(template: &str, options: JsValue) -> Result<String, String> {
    let mut parser = Parser::builder()
        .options(&Options::from_js(options)?)?
        .build()?;

    // UTF-16 offset of each byte offset that starts a character (or ends
    // the text).
    let mut utf16 = vec![0u32; template.len() + 1];
    let mut units = 0;
    for (i, c) in template.char_indices() {
        utf16[i] = units;
        units += c.len_utf16() as u32;
    }
    utf16[template.len()] = units;
    let at = |offset: u32| utf16[offset as usize];

    let tokens: Vec<_> = parser
        .highlight(template)?
        .into_iter()
        .map(|(span, kind)| serde_json::json!([at(span.start), at(span.end), kind.name()]))
        .collect();
    let error = match parser.parse(template, None) {
        Ok(_) => serde_json::Value::Null,
        Err(message) => {
            let span = parser.error_span.unwrap_or(Span::at(template.len()));
            serde_json::json!({ "message": message, "start": at(span.start), "end": at(span.end) })
        }
    };
    Ok(serde_json::json!({ "tokens": tokens, "error": error }).to_string())
}

/// The Template Toolkit version this port follows.
#[wasm_bindgen]
pub fn version() -> String {
    "Template Toolkit 3.009 (Rust port)".to_string()
}
