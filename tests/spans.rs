//! Token spans point at the source they came from, checked over every
//! upstream test case in several parser styles: each directive token's
//! span holds the token's own text, highlighting covers the source in
//! order without overlaps, and parse errors point inside the source.
mod common;
use common::TT_DIST;
use template_toolkit::{
    constants::{CHOMP_COLLAPSE, CHOMP_GREEDY, CHOMP_ONE},
    parser::{
        highlight::highlight,
        lexer::{split_text, ParserStyle, Span, SplitChunk},
        Parser,
    },
};

fn cases() -> Vec<(String, String)> {
    fn walk(dir: &std::path::Path, out: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "t") {
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let Some(start) = text
                    .find("\n__DATA__\n")
                    .or_else(|| text.find("\n__END__\n"))
                else {
                    continue;
                };
                for (i, case) in text[start..].split("-- test --").enumerate().skip(1) {
                    let input = case.split("-- expect --").next().unwrap();
                    out.push((format!("{}#{i}", path.display()), input.to_string()));
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(&std::path::Path::new(TT_DIST).join("t"), &mut out);
    assert!(out.len() > 1000, "too few cases ({})", out.len());
    out
}

fn styles() -> Vec<(&'static str, ParserStyle)> {
    let d = ParserStyle::default;
    vec![
        ("default", d()),
        (
            "interpolate",
            ParserStyle {
                interpolate: true,
                ..d()
            },
        ),
        (
            "anycase",
            ParserStyle {
                anycase: true,
                ..d()
            },
        ),
        (
            "chomp one",
            ParserStyle {
                pre_chomp: CHOMP_ONE,
                post_chomp: CHOMP_ONE,
                ..d()
            },
        ),
        (
            "collapse",
            ParserStyle {
                pre_chomp: CHOMP_COLLAPSE,
                post_chomp: CHOMP_COLLAPSE,
                interpolate: true,
                ..d()
            },
        ),
        (
            "greedy",
            ParserStyle {
                pre_chomp: CHOMP_GREEDY,
                post_chomp: CHOMP_GREEDY,
                ..d()
            },
        ),
    ]
}

/// The source a token came from, for tokens whose value is it verbatim.
fn verbatim(kind: &str) -> bool {
    !matches!(kind, "\"" | "TEXT" | "LITERAL" | "QUOTED")
}

fn check_span(text: &str, span: Span, what: &str) {
    assert!(
        span.start <= span.end && span.end as usize <= text.len(),
        "{what}: {span:?} outside 0..{}",
        text.len()
    );
    assert!(
        text.is_char_boundary(span.start as usize) && text.is_char_boundary(span.end as usize),
        "{what}: {span:?}"
    );
}

#[test]
fn token_spans_hold_their_tokens() {
    let mut checked = 0;
    for (name, text) in cases() {
        for (style_name, style) in styles() {
            let what = format!("{name} ({style_name})");
            let Ok(chunks) = split_text(&text, &style) else {
                continue;
            };
            for chunk in chunks {
                let SplitChunk::Directive {
                    tokens,
                    spans,
                    open,
                    close,
                    ..
                } = chunk
                else {
                    continue;
                };
                assert_eq!(tokens.len(), spans.len(), "{what}");
                check_span(&text, open, &what);
                check_span(&text, close, &what);
                for ((kind, value), span) in tokens.iter().zip(&spans) {
                    check_span(&text, *span, &what);
                    let source = &text[span.range()];
                    if verbatim(kind) && *kind != ";" {
                        let same = if style.anycase {
                            source.eq_ignore_ascii_case(value)
                        } else {
                            source == value
                        };
                        assert!(same, "{what}: {kind} {value:?} at {span:?} is {source:?}");
                        checked += 1;
                    }
                    if *kind == "LITERAL" && !source.is_empty() {
                        assert!(
                            source.starts_with(['\'', '"']),
                            "{what}: LITERAL {value:?} at {span:?} is {source:?}"
                        );
                    }
                }
            }
        }
    }
    assert!(checked > 10_000, "only {checked} tokens checked");
}

#[test]
fn highlighting_is_ordered_and_disjoint() {
    for (name, text) in cases() {
        for (style_name, style) in styles() {
            let what = format!("{name} ({style_name})");
            let Ok(marks) = highlight(&text, &style) else {
                continue;
            };
            let mut last = 0;
            for (span, kind) in marks {
                check_span(&text, span, &what);
                assert!(
                    span.start >= last && span.start < span.end,
                    "{what}: {kind:?} {span:?} after {last}"
                );
                last = span.end;
            }
        }
    }
}

#[test]
fn parse_errors_point_into_the_source() {
    let mut errors = 0;
    for (name, text) in cases() {
        let mut parser = Parser::new();
        if parser.parse(&text, None).is_err() {
            let span = parser
                .error_span
                .unwrap_or_else(|| panic!("{name}: no error span for {}", parser.error_msg));
            check_span(&text, span, &name);
            errors += 1;
        }
    }
    assert!(errors > 0);
}

#[test]
fn parse_error_spans() {
    fn at(text: &str) -> (String, Option<&str>) {
        let mut parser = Parser::new();
        let err = parser.parse(text, None).unwrap_err();
        (
            err.lines().next().unwrap().to_string(),
            parser.error_span.map(|s| &text[s.range()]),
        )
    }
    assert_eq!(
        at("a [% foo = = 1 %] b"),
        ("line 1: unexpected token (=)".into(), Some("="))
    );
    assert_eq!(
        at("[% IF x %]yes"),
        ("line 1: unexpected end of input".into(), Some(""))
    );
    assert_eq!(
        at("x\n[% FOREACH -%]"),
        ("line 2: unexpected end of directive".into(), Some("-%]"))
    );
    assert_eq!(
        at(r#"[% "a $b c" d %]"#),
        ("line 1: unexpected token (d)".into(), Some("d"))
    );
}
