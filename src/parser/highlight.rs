//! Syntax highlighting from the lexer itself: what each part of a template
//! is, by byte span, exactly as the parser will see it (tag styles, TAGS
//! directives, ANYCASE, INTERPOLATE, chomp flags and `$var` in strings
//! included).

use crate::parser::lexer::{
    interpolate_text_at, split_text, CowStr, InertKind, ParserStyle, SourceAt, Span, SplitChunk,
};

/// What a span of a template is.  Spans of plain text are not reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lexeme {
    /// A start or end tag, with its chomp flag.
    Tag,
    /// A comment directive, or a `#` comment inside a directive.
    Comment,
    /// A directive keyword or word operator: `IF`, `FOREACH`, `AND`, ...
    Keyword,
    /// A variable or other identifier.
    Variable,
    /// A quoted string, or the literal text of one with variables in it.
    String,
    /// The `$`, `${` and `}` around a variable in a string or in text.
    Interpolation,
    Number,
    /// A bare filename: `INCLUDE header.tt`.
    Filename,
    /// Punctuation and symbolic operators.
    Operator,
}

impl Lexeme {
    pub fn name(self) -> &'static str {
        match self {
            Lexeme::Tag => "tag",
            Lexeme::Comment => "comment",
            Lexeme::Keyword => "keyword",
            Lexeme::Variable => "variable",
            Lexeme::String => "string",
            Lexeme::Interpolation => "interpolation",
            Lexeme::Number => "number",
            Lexeme::Filename => "filename",
            Lexeme::Operator => "operator",
        }
    }
}

/// The highlighted spans of `text` in `style`, in order and not
/// overlapping.  Fails only where the parser would, on an invalid tag style.
pub fn highlight(text: &str, style: &ParserStyle) -> Result<Vec<(Span, Lexeme)>, String> {
    let mut out = Vec::new();
    for chunk in split_text(text, style)? {
        match chunk {
            SplitChunk::Text(..) | SplitChunk::Token(..) => {}
            SplitChunk::IText {
                text: itext,
                span,
                origin,
                ..
            } => {
                let at = SourceAt::Shift {
                    origin,
                    lo: span.start as usize,
                    hi: span.end as usize,
                };
                for sub in interpolate_text_at(&itext, 0, style.anycase, at) {
                    if let SplitChunk::Directive {
                        tokens,
                        spans,
                        open,
                        close,
                        ..
                    } = sub
                    {
                        push(&mut out, open, Lexeme::Interpolation);
                        directive(text, &tokens, &spans, open.end, close.start, &mut out);
                        push(&mut out, close, Lexeme::Interpolation);
                    }
                }
            }
            SplitChunk::Inert {
                kind: InertKind::Comment,
                open,
                close,
                ..
            } => {
                push(
                    &mut out,
                    Span {
                        start: open.start,
                        end: close.end,
                    },
                    Lexeme::Comment,
                );
            }
            SplitChunk::Inert {
                kind: InertKind::Tags,
                open,
                body,
                close,
            } => {
                push(&mut out, open, Lexeme::Tag);
                push(&mut out, body, Lexeme::Keyword);
                push(&mut out, close, Lexeme::Tag);
            }
            SplitChunk::Directive {
                tokens,
                spans,
                open,
                close,
                ..
            } => {
                push(&mut out, open, Lexeme::Tag);
                directive(text, &tokens, &spans, open.end, close.start, &mut out);
                push(&mut out, close, Lexeme::Tag);
            }
        }
    }
    Ok(out)
}

fn push(out: &mut Vec<(Span, Lexeme)>, span: Span, kind: Lexeme) {
    if span.start < span.end {
        out.push((span, kind));
    }
}

/// A directive's tokens, between `start` and `end`.  What lies between
/// tokens is whitespace, a `#` comment, or (in a string) the `$`, `${` and
/// `}` of an interpolated variable.
fn directive(
    text: &str,
    tokens: &[(&'static str, CowStr<'_>)],
    spans: &[Span],
    start: u32,
    end: u32,
    out: &mut Vec<(Span, Lexeme)>,
) {
    let mut gap_start = start;
    let mut in_string = false;
    for ((kind, _), &span) in tokens.iter().zip(spans) {
        if span.start < gap_start || span.start == span.end {
            continue;
        }
        gap(
            text,
            Span {
                start: gap_start,
                end: span.start,
            },
            in_string,
            out,
        );
        let lexeme = match *kind {
            "\"" => {
                in_string = !in_string;
                Lexeme::String
            }
            "IDENT" => Lexeme::Variable,
            "LITERAL" | "TEXT" => Lexeme::String,
            "NUMBER" => Lexeme::Number,
            "FILENAME" => Lexeme::Filename,
            _ if text[span.range()].starts_with(|c: char| c.is_alphabetic()) => Lexeme::Keyword,
            _ => Lexeme::Operator,
        };
        push(out, span, lexeme);
        gap_start = span.end;
    }
    gap(
        text,
        Span {
            start: gap_start,
            end,
        },
        in_string,
        out,
    );
}

fn gap(text: &str, span: Span, in_string: bool, out: &mut Vec<(Span, Lexeme)>) {
    if span.start >= span.end {
        return;
    }
    if in_string {
        push(out, span, Lexeme::Interpolation);
        return;
    }
    // Whitespace, and comments running from `#` to the end of the line.
    let slice = &text[span.range()];
    let mut from = 0;
    while let Some(hash) = slice[from..].find('#').map(|i| from + i) {
        let eol = slice[hash..].find('\n').map_or(slice.len(), |i| hash + i);
        push(
            out,
            Span {
                start: span.start + hash as u32,
                end: span.start + eol as u32,
            },
            Lexeme::Comment,
        );
        from = eol;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str, style: &ParserStyle) -> Vec<(String, &'static str)> {
        highlight(text, style)
            .unwrap()
            .into_iter()
            .map(|(s, k)| (text[s.range()].to_string(), k.name()))
            .collect()
    }

    fn pairs(list: &[(&str, &'static str)]) -> Vec<(String, &'static str)> {
        list.iter().map(|(t, k)| (t.to_string(), *k)).collect()
    }

    #[test]
    fn directive_parts() {
        let style = ParserStyle::default();
        assert_eq!(
            kinds("a [%- IF user.name == 'x' # why\n %]b[% END -%]", &style),
            pairs(&[
                ("[%-", "tag"),
                ("IF", "keyword"),
                ("user", "variable"),
                (".", "operator"),
                ("name", "variable"),
                ("==", "operator"),
                ("'x'", "string"),
                ("# why", "comment"),
                ("%]", "tag"),
                ("[%", "tag"),
                ("END", "keyword"),
                ("-%]", "tag"),
            ])
        );
    }

    #[test]
    fn strings_with_variables() {
        let style = ParserStyle::default();
        assert_eq!(
            kinds(r#"[% x = "a\"b $c ${d.e}" %]"#, &style),
            pairs(&[
                ("[%", "tag"),
                ("x", "variable"),
                ("=", "operator"),
                ("\"", "string"),
                ("a\\\"b ", "string"),
                ("$", "interpolation"),
                ("c", "variable"),
                (" ", "string"),
                ("${", "interpolation"),
                ("d", "variable"),
                (".", "operator"),
                ("e", "variable"),
                ("}", "interpolation"),
                ("\"", "string"),
                ("%]", "tag"),
            ])
        );
    }

    #[test]
    fn comments_tags_and_interpolated_text() {
        let style = ParserStyle {
            interpolate: true,
            ..ParserStyle::default()
        };
        assert_eq!(
            kinds("[%# note %]Hi $name![% TAGS star %][* x *]", &style),
            pairs(&[
                ("[%# note %]", "comment"),
                ("$", "interpolation"),
                ("name", "variable"),
                ("[%", "tag"),
                ("TAGS star", "keyword"),
                ("%]", "tag"),
                ("[*", "tag"),
                ("x", "variable"),
                ("*]", "tag"),
            ])
        );
    }

    #[test]
    fn collapsing_chomp_keeps_offsets() {
        // POST_CHOMP collapse puts a space in front of the next text, which
        // must not shift what follows.
        let style = ParserStyle {
            post_chomp: crate::constants::CHOMP_COLLAPSE,
            interpolate: true,
            ..ParserStyle::default()
        };
        assert_eq!(
            kinds("[% a %]\n  $b [% c %]", &style),
            pairs(&[
                ("[%", "tag"),
                ("a", "variable"),
                ("%]", "tag"),
                ("$", "interpolation"),
                ("b", "variable"),
                ("[%", "tag"),
                ("c", "variable"),
                ("%]", "tag"),
            ])
        );
    }
}
