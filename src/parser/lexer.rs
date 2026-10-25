use std::{collections::HashSet, sync::LazyLock};

use regex::RegexBuilder;
use regex_automata::{meta, util::syntax, Anchored, Input};

use crate::{
    constants::{CHOMP_ALL, CHOMP_COLLAPSE, CHOMP_GREEDY, CHOMP_NONE},
    parser::grammar_tables::lookup_lextable,
    regex_cache,
};

#[derive(Debug, Clone)]
pub enum SplitChunk {
    Text(String),
    IText {
        text: String,
        line: usize,
    },
    Directive {
        text: String,
        line: String,
        tokens: Vec<(String, String)>,
    },
    Token(String, String),
}

#[derive(Debug, Clone)]
pub struct ParserStyle {
    pub start_tag: String,
    pub end_tag: String,
    pub outline_tag: Option<String>,
    pub anycase: bool,
    pub interpolate: bool,
    pub pre_chomp: usize,
    pub post_chomp: usize,
    pub v1dollar: bool,
    pub eval_perl: bool,
}

impl Default for ParserStyle {
    fn default() -> Self {
        Self {
            start_tag: r"\[%".to_string(),
            end_tag: r"%\]".to_string(),
            outline_tag: None,
            anycase: false,
            interpolate: false,
            pre_chomp: CHOMP_NONE,
            post_chomp: CHOMP_NONE,
            v1dollar: false,
            eval_perl: false,
        }
    }
}

pub fn get_tag_style(name: &str) -> Option<(&'static str, &'static str, Option<&'static str>)> {
    match name {
        "outline" => Some((r"\[%", r"%\]", Some("%%"))),
        "default" | "template" | "tt2" => Some((r"\[%", r"%\]", None)),
        "template1" => Some((r"[\[%]%", r"%[\]%]", None)),
        "metatext" => Some(("%%", "%%", None)),
        "html" => Some(("<!--", "-->", None)),
        "mason" => Some(("<%", ">", None)),
        "asp" => Some(("<%", "%>", None)),
        "php" => Some((r"<\?", r"\?>", None)),
        "star" => Some((r"\[\*", r"\*\]", None)),
        _ => None,
    }
}

/// Finds the next directive, replacing Perl's `text_splitter` regex
///
/// ```text
/// \A(.*?)(?: (?:^$out(.*?)(?:\n|$)) | (?:$start(.*?)$end) )   /msx
/// ```
///
/// with capture-free `find`s.  Capture groups force the regex crate onto
/// its slower engines over the whole match, which here includes all the
/// text before the tag; plain `find` runs on the DFA with literal
/// prefilters (memchr/memmem for the usual literal tags).
pub struct Splitter {
    start: regex::Regex,
    end: regex::Regex,
    outline: Option<regex::Regex>,
}

/// Byte offsets of one directive within the searched text.
struct Split {
    /// End of the text preceding the tag (`$1`).
    pre_end: usize,
    /// The directive contents (`$2` / `$3`).
    dir: std::ops::Range<usize>,
    /// End of the whole match, i.e. how much text was consumed.
    end: usize,
}

impl Splitter {
    pub fn new(start: &str, end: &str, outline: Option<&str>) -> Result<Self, String> {
        let compile = |pat: String, multi_line: bool| {
            let flags = regex_cache::Flags {
                multi_line,
                dot_matches_new_line: true,
                ignore_whitespace: true,
            };
            regex_cache::regex_with(&pat, flags).ok_or_else(|| {
                // Recompile uncached to report why the pattern is invalid.
                RegexBuilder::new(&pat)
                    .build()
                    .err()
                    .map(|e| e.to_string())
                    .unwrap_or_default()
            })
        };
        Ok(Self {
            start: compile(format!("(?:{start})"), false)?,
            end: compile(format!("(?:{end})"), false)?,
            outline: outline
                .map(|out| compile(format!("^(?:{out})"), true))
                .transpose()?,
        })
    }

    /// The leftmost directive in `hay`, preferring an outline tag when both
    /// kinds start at the same place (the regex alternation's order).
    fn next(&self, hay: &str) -> Option<Split> {
        let outline = |m: regex::Match| {
            let eol = memchr::memchr(b'\n', &hay.as_bytes()[m.end()..]).map(|i| m.end() + i);
            Split {
                pre_end: m.start(),
                dir: m.end()..eol.unwrap_or(hay.len()),
                end: eol.map_or(hay.len(), |i| i + 1),
            }
        };
        let tag = self.start.find(hay);
        let out = self.outline.as_ref().and_then(|re| re.find(hay));
        match (tag, out) {
            (Some(t), Some(o)) if o.start() <= t.start() => Some(outline(o)),
            (Some(t), o) => match self.end.find_at(hay, t.end()) {
                Some(e) => Some(Split {
                    pre_end: t.start(),
                    dir: t.end()..e.start(),
                    end: e.end(),
                }),
                // No end tag after the first start tag means none after any
                // later one either, so only an outline tag can still match.
                None => o.map(outline),
            },
            (None, o) => o.map(outline),
        }
    }
}

fn chomp_char_to_mode(c: char) -> Option<usize> {
    match c {
        '-' => Some(CHOMP_ALL),
        '=' => Some(CHOMP_COLLAPSE),
        '~' => Some(CHOMP_GREEDY),
        '+' => Some(CHOMP_NONE),
        _ => None,
    }
}

static PRE_CHOMP_ALL_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    RegexBuilder::new(r"(\r?\n|^)[^\S\n]*\z")
        .multi_line(true)
        .build()
        .unwrap()
});

static PRE_CHOMP_WS_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\s+\z").unwrap());

static POST_CHOMP_ALL_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\A([^\S\n]*\n)").unwrap());

static POST_CHOMP_WS_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\A(\s+)").unwrap());

static ANYCASE_AFTER: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "GET", "SET", "CALL", "DEFAULT", "INSERT", "INCLUDE", "PROCESS", "WRAPPER", "BLOCK", "USE",
        "PLUGIN", "FILTER", "MACRO", "IN", "TO", "STEP", "AND", "OR", "NOT", "DIV", "MOD", "DOT",
        "IF", "UNLESS", "ELSIF", "FOR", "WHILE", "SWITCH", "CASE", "META", "THROW", "CATCH",
        "VIEW", "CMPOP", "BINOP", "COMMA", "(", "[", "{",
    ]
    .into_iter()
    .collect()
});

pub fn split_text(text: &str, style: &ParserStyle) -> Result<Vec<SplitChunk>, String> {
    let mut tokens = Vec::new();
    if text.is_empty() {
        return Ok(tokens);
    }

    let mut start = style.start_tag.clone();
    let mut end = style.end_tag.clone();
    let mut out = style.outline_tag.clone();
    let prechomp = style.pre_chomp;
    let postchomp = style.post_chomp;
    let interp = style.interpolate;
    let anycase = style.anycase;

    let mut splitter = Splitter::new(&start, &end, out.as_deref())?;
    // Perl consumes `$text` with s///, so each search starts afresh at the
    // start of what is left (which matters for the outline tag's `^`).  We
    // track that as an offset into `text`, plus whether a collapsing
    // post-chomp has left a single space in front of it.
    let mut pos = 0;
    let mut lead_space = false;
    let mut line: usize = 1;

    loop {
        let spaced;
        let hay: &str = if lead_space {
            spaced = format!(" {}", &text[pos..]);
            &spaced
        } else {
            &text[pos..]
        };
        let Some(split) = splitter.next(hay) else {
            break;
        };
        let mut pre = hay[..split.pre_end].to_string();
        let mut dir = hay[split.dir.clone()].to_string();
        pos += split.end - usize::from(lead_space);
        lead_space = false;

        let mut prelines = pre.bytes().filter(|&b| b == b'\n').count();
        let mut dirlines = dir.bytes().filter(|&b| b == b'\n').count();
        let mut postlines = 0;

        if dir.starts_with('#') {
            // Comment out entire directive except for any end chomp flag
            if let Some(last_ch) = dir.chars().last() {
                if chomp_char_to_mode(last_ch).is_some() {
                    dir = last_ch.to_string();
                } else {
                    dir.clear();
                }
            } else {
                dir.clear();
            }
        } else {
            // Check leading chomp flag and whitespace: s/^($CHOMP_FLAGS)?(\s*)//so
            let mut chars = dir.chars();
            let first_ch = chars.next();
            let pre_flag = first_ch.and_then(chomp_char_to_mode);
            let ws_start = if pre_flag.is_some() {
                first_ch.map(|c| c.len_utf8()).unwrap_or(0)
            } else {
                0
            };
            let ws_part: String = dir[ws_start..]
                .chars()
                .take_while(|c| c.is_whitespace())
                .collect();
            let ws_len = ws_part.len();
            if ws_len > 0 {
                let linecount = ws_part.bytes().filter(|&b| b == b'\n').count();
                prelines += linecount;
                dirlines = dirlines.saturating_sub(linecount);
            }
            dir.drain(..(ws_start + ws_len));

            let chomp = pre_flag.unwrap_or(prechomp);
            if chomp != CHOMP_NONE && !pre.is_empty() {
                match chomp {
                    CHOMP_ALL => {
                        pre = PRE_CHOMP_ALL_RE.replace(&pre, "").into_owned();
                    }
                    CHOMP_COLLAPSE => {
                        pre = PRE_CHOMP_WS_RE.replace(&pre, " ").into_owned();
                    }
                    CHOMP_GREEDY => {
                        pre = PRE_CHOMP_WS_RE.replace(&pre, "").into_owned();
                    }
                    _ => {}
                }
            }
        }

        // POST_CHOMP: s/\s*($CHOMP_FLAGS)?\s*$//so
        let trimmed_end = dir.trim_end();
        let mut post_flag = None;
        let new_dir_len = if let Some(last_ch) = trimmed_end.chars().last() {
            if let Some(mode) = chomp_char_to_mode(last_ch) {
                post_flag = Some(mode);
                let without_flag = &trimmed_end[..trimmed_end.len() - last_ch.len_utf8()];
                without_flag.trim_end().len()
            } else {
                trimmed_end.len()
            }
        } else {
            0
        };
        dir.truncate(new_dir_len);

        let chomp = post_flag.unwrap_or(postchomp);
        let remaining = &text[pos..];
        if chomp != CHOMP_NONE && !remaining.is_empty() {
            match chomp {
                CHOMP_ALL => {
                    if let Some(m) = POST_CHOMP_ALL_RE.find(remaining) {
                        pos += m.end();
                        postlines += 1;
                    }
                }
                CHOMP_COLLAPSE => {
                    if let Some(m) = POST_CHOMP_WS_RE.find(remaining) {
                        postlines += m.as_str().bytes().filter(|&b| b == b'\n').count();
                        pos += m.end();
                        lead_space = true;
                    }
                }
                CHOMP_GREEDY => {
                    if let Some(m) = POST_CHOMP_WS_RE.find(remaining) {
                        postlines += m.as_str().bytes().filter(|&b| b == b'\n').count();
                        pos += m.end();
                    }
                }
                _ => {}
            }
        }

        if !pre.is_empty() {
            if interp {
                tokens.push(SplitChunk::IText { text: pre, line });
            } else {
                tokens.push(SplitChunk::Text(pre));
            }
        }
        line += prelines;

        if !dir.is_empty() {
            // Check TAGS directive
            let is_tags = if anycase {
                dir.len() > 4
                    && dir[..4].eq_ignore_ascii_case("TAGS")
                    && dir[4..].starts_with(char::is_whitespace)
            } else {
                dir.starts_with("TAGS") && dir[4..].starts_with(char::is_whitespace)
            };

            if is_tags {
                let rest = dir[4..].trim();
                let parts: Vec<&str> = rest.split_whitespace().collect();
                if parts.len() > 1 {
                    start = regex::escape(parts[0]);
                    end = regex::escape(parts[1]);
                    out = parts.get(2).map(|s| regex::escape(s));
                    splitter = Splitter::new(&start, &end, out.as_deref())?;
                } else if parts.len() == 1 {
                    if let Some((s, e, o)) = get_tag_style(parts[0]) {
                        start = s.to_string();
                        end = e.to_string();
                        out = o.map(|x| x.to_string());
                        splitter = Splitter::new(&start, &end, out.as_deref())?;
                    }
                }
            } else {
                let line_str = if dirlines > 0 {
                    format!("{}-{}", line, line + dirlines)
                } else {
                    line.to_string()
                };
                let dir_tokens = tokenise_directive(&dir, anycase);
                tokens.push(SplitChunk::Directive {
                    text: dir,
                    line: line_str,
                    tokens: dir_tokens,
                });
            }
        }

        line += dirlines + postlines;
    }

    let remaining = if lead_space {
        format!(" {}", &text[pos..])
    } else {
        text[pos..].to_string()
    };
    if !remaining.is_empty() {
        if interp {
            tokens.push(SplitChunk::IText {
                text: remaining,
                line,
            });
        } else {
            tokens.push(SplitChunk::Text(remaining));
        }
    }

    Ok(tokens)
}

/// Build a multi-pattern regex for capture-free tokenising: a search
/// reports which pattern matched (leftmost-first, ties to the earlier
/// pattern, just like one alternation), so the patterns need no capture
/// groups and `search` can stay on the DFA engines.
fn multi_regex(patterns: &[&str]) -> meta::Regex {
    meta::Regex::builder()
        .syntax(syntax::Config::new().dot_matches_new_line(true))
        .build_many(patterns)
        .unwrap()
}

/// Perl's `((?:\\.|[^\$]){1,3000})|(\$(?:(?:\{([^\}]*)\})|([\w\.]+)))`, split
/// into its alternatives.  Perl bounds the text run at {1,3000} to dodge
/// its regex recursion limit; regex-automata would expand that into 3000
/// copies of the sub-NFA, and an unbounded run only merges what Perl emits
/// as adjacent text chunks, so `+` gives identical output.
const INTERP_TEXT: usize = 0;
const INTERP_BRACED: usize = 1;
static INTERP_RE: LazyLock<meta::Regex> =
    LazyLock::new(|| multi_regex(&[r"(?:\\.|[^\$])+", r"\$\{[^\}]*\}", r"\$[\w\.]+"]));

pub fn interpolate_text(text: &str, mut line: usize, anycase: bool) -> Vec<SplitChunk> {
    let mut chunks = Vec::new();
    let mut pos = 0;
    let bytes = text.as_bytes();

    while pos < text.len() {
        if let Some(m) = INTERP_RE.search(&Input::new(text).span(pos..text.len())) {
            if m.start() > pos {
                // Unmatched '$' before match
                let skipped = &text[pos..m.start()];
                line += skipped.bytes().filter(|&b| b == b'\n').count();
                chunks.push(SplitChunk::Text(skipped.to_string()));
                pos = m.start();
                continue;
            }
            pos = m.end();
            let matched = &text[m.range()];

            if m.pattern().as_usize() == INTERP_TEXT {
                line += matched.bytes().filter(|&b| b == b'\n').count();
                chunks.push(SplitChunk::Text(matched.replace(r"\$", "$")));
            } else {
                let var = if m.pattern().as_usize() == INTERP_BRACED {
                    &matched[2..matched.len() - 1]
                } else {
                    &matched[1..]
                };
                line += matched.bytes().filter(|&b| b == b'\n').count();
                if !var.is_empty() {
                    let dir_tokens = tokenise_directive(var, anycase);
                    chunks.push(SplitChunk::Directive {
                        text: matched.to_string(),
                        line: line.to_string(),
                        tokens: dir_tokens,
                    });
                } else {
                    chunks.push(SplitChunk::Text(matched.to_string()));
                }
            }
        } else {
            // Remaining lone '$'
            let ch_len = text[pos..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
            let slice = &text[pos..pos + ch_len];
            if bytes[pos] == b'\n' {
                line += 1;
            }
            chunks.push(SplitChunk::Text(slice.to_string()));
            pos += ch_len;
        }
    }

    chunks
}

/// The alternatives of Perl's tokenise_directive regex (after quoted
/// strings), in order; the matching pattern's index replaces its captures.
const TOK_SKIP: usize = 0;
const TOK_NUMBER: usize = 1;
const TOK_FILENAME: usize = 2;
const TOK_WORD: usize = 3;
static TOKEN_REST_RE: LazyLock<meta::Regex> = LazyLock::new(|| {
    multi_regex(&[
        r"\s+|\#[^\n]*",
        r"-?\d+(?:\.\d+)?",
        r"\/?\w+(?:(?:\/|::?)\w*)+|\/\w+",
        r"\w+",
        r"[(){}\[\]:;,\/\\]|[+\-*]|\$\{?|=>|[=!<>]?=|[!<>]|&&?|\|\|?|\.\.?|\S+",
    ])
});

static ANYCASE_BEFORE_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\A\s*[=\.]").unwrap());

pub fn tokenise_directive(text: &str, anycase: bool) -> Vec<(String, String)> {
    let mut tokens: Vec<(String, String)> = Vec::new();
    let mut pos = 0;
    let bytes = text.as_bytes();

    while pos < text.len() {
        let b = bytes[pos];
        // Handle quoted strings: "..." or '...'
        if b == b'"' || b == b'\'' {
            let quote = b;
            let mut end_pos = pos + 1;
            let mut closed = false;
            while end_pos < bytes.len() {
                if bytes[end_pos] == b'\\'
                    && end_pos + 1 < bytes.len()
                    && (bytes[end_pos + 1] == b'\\' || bytes[end_pos + 1] == quote)
                {
                    end_pos += 2;
                    continue;
                }
                if bytes[end_pos] == quote {
                    closed = true;
                    break;
                }
                end_pos += 1;
            }
            if closed {
                let inner = &text[pos + 1..end_pos];
                pos = end_pos + 1;
                if quote == b'"' {
                    if inner.contains('$') || inner.contains('\\') {
                        // Process escapes:
                        // s/\\([^\$nrt])/$1/g;
                        // s/\\([nrt])/$QUOTED_ESCAPES->{ $1 }/ge;
                        let mut processed = String::with_capacity(inner.len());
                        let mut chars = inner.chars().peekable();
                        while let Some(c) = chars.next() {
                            if c == '\\' {
                                if let Some(&next_c) = chars.peek() {
                                    match next_c {
                                        '$' => {
                                            processed.push('\\');
                                            processed.push('$');
                                            chars.next();
                                        }
                                        'n' => {
                                            processed.push('\n');
                                            chars.next();
                                        }
                                        'r' => {
                                            processed.push('\r');
                                            chars.next();
                                        }
                                        't' => {
                                            processed.push('\t');
                                            chars.next();
                                        }
                                        other => {
                                            processed.push(other);
                                            chars.next();
                                        }
                                    }
                                } else {
                                    processed.push('\\');
                                }
                            } else {
                                processed.push(c);
                            }
                        }

                        tokens.push(("\"".to_string(), "\"".to_string()));
                        let interp_chunks = interpolate_text(&processed, 1, anycase);
                        for chunk in interp_chunks {
                            match chunk {
                                SplitChunk::Text(t) => {
                                    tokens.push(("TEXT".to_string(), t));
                                }
                                SplitChunk::Directive {
                                    tokens: sub_tokens, ..
                                } => {
                                    tokens.extend(sub_tokens);
                                    tokens.push((";".to_string(), ";".to_string()));
                                }
                                _ => {}
                            }
                        }
                        tokens.push(("\"".to_string(), "\"".to_string()));
                    } else {
                        let escaped = inner.replace('\'', r"\'");
                        tokens.push(("LITERAL".to_string(), format!("'{}'", escaped)));
                    }
                } else {
                    tokens.push(("LITERAL".to_string(), format!("'{}'", inner)));
                }
                continue;
            }
        }

        let input = Input::new(text)
            .span(pos..text.len())
            .anchored(Anchored::Yes);
        if let Some(m) = TOKEN_REST_RE.search(&input) {
            pos = m.end();
            let token = &text[m.range()];
            let kind = m.pattern().as_usize();

            if kind == TOK_SKIP {
                // whitespace or comment
            } else if kind == TOK_NUMBER {
                tokens.push(("NUMBER".to_string(), token.to_string()));
            } else if kind == TOK_FILENAME {
                tokens.push(("FILENAME".to_string(), token.to_string()));
            } else if kind == TOK_WORD {
                let uctoken = if anycase {
                    let prev_blocks_fold = tokens
                        .last()
                        .map(|(t, _)| ANYCASE_AFTER.contains(t.as_str()))
                        .unwrap_or(false);
                    let next_blocks_fold = ANYCASE_BEFORE_RE.is_match(&text[pos..]);
                    if prev_blocks_fold || next_blocks_fold {
                        token.to_string()
                    } else {
                        token.to_ascii_uppercase()
                    }
                } else {
                    token.to_string()
                };

                if let Some(lex_type) = lookup_lextable(&uctoken) {
                    tokens.push((lex_type.to_string(), uctoken));
                } else {
                    tokens.push(("IDENT".to_string(), token.to_string()));
                }
            } else {
                let uctoken = if anycase {
                    token.to_ascii_uppercase()
                } else {
                    token.to_string()
                };
                if let Some(lex_type) = lookup_lextable(&uctoken) {
                    tokens.push((lex_type.to_string(), token.to_string()));
                } else {
                    tokens.push(("UNQUOTED".to_string(), token.to_string()));
                }
            }
        } else {
            // Advance by 1 char if nothing matched
            let ch_len = text[pos..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
            pos += ch_len;
        }
    }

    tokens
}
