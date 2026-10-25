use std::{collections::HashSet, sync::LazyLock};

use regex::RegexBuilder;

use crate::{
    constants::{CHOMP_ALL, CHOMP_COLLAPSE, CHOMP_GREEDY, CHOMP_NONE},
    parser::grammar_tables::lookup_lextable,
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

pub fn build_splitter_regex(
    start: &str,
    end: &str,
    outline: Option<&str>,
) -> Result<regex::Regex, String> {
    if let Some(out) = outline {
        let pat = format!(r"\A(.*?)(?:(?:^{out}(.*?)(?:\n|\z))|(?:{start}(.*?){end}))");
        RegexBuilder::new(&pat)
            .multi_line(true)
            .dot_matches_new_line(true)
            .build()
            .map_err(|e| e.to_string())
    } else {
        let pat = format!(r"\A(.*?)(?:{start}(.*?){end})");
        RegexBuilder::new(&pat)
            .dot_matches_new_line(true)
            .build()
            .map_err(|e| e.to_string())
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

    let mut split_re = build_splitter_regex(&start, &end, out.as_deref())?;
    let mut remaining = text.to_string();
    let mut line: usize = 1;

    while let Some(caps) = split_re.captures(&remaining) {
        let full_match_len = caps.get(0).unwrap().end();
        let mut pre = caps
            .get(1)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        let mut dir = if out.is_some() {
            caps.get(2)
                .or_else(|| caps.get(3))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default()
        } else {
            caps.get(2)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default()
        };

        remaining.drain(..full_match_len);

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
        if chomp != CHOMP_NONE && !remaining.is_empty() {
            match chomp {
                CHOMP_ALL => {
                    if let Some(m) = POST_CHOMP_ALL_RE.find(&remaining) {
                        let m_end = m.end();
                        remaining.drain(..m_end);
                        postlines += 1;
                    }
                }
                CHOMP_COLLAPSE => {
                    if let Some(m) = POST_CHOMP_WS_RE.find(&remaining) {
                        let nl_count = m.as_str().bytes().filter(|&b| b == b'\n').count();
                        postlines += nl_count;
                        let m_end = m.end();
                        remaining.replace_range(..m_end, " ");
                    }
                }
                CHOMP_GREEDY => {
                    if let Some(m) = POST_CHOMP_WS_RE.find(&remaining) {
                        let nl_count = m.as_str().bytes().filter(|&b| b == b'\n').count();
                        postlines += nl_count;
                        let m_end = m.end();
                        remaining.drain(..m_end);
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
                    split_re = build_splitter_regex(&start, &end, out.as_deref())?;
                } else if parts.len() == 1 {
                    if let Some((s, e, o)) = get_tag_style(parts[0]) {
                        start = s.to_string();
                        end = e.to_string();
                        out = o.map(|x| x.to_string());
                        split_re = build_splitter_regex(&start, &end, out.as_deref())?;
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

static INTERP_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    RegexBuilder::new(r"((?:\\.|[^\$]){1,3000})|(\$(?:(?:\{([^\}]*)\})|([\w\.]+)))")
        .dot_matches_new_line(true)
        .build()
        .unwrap()
});

pub fn interpolate_text(text: &str, mut line: usize, anycase: bool) -> Vec<SplitChunk> {
    let mut chunks = Vec::new();
    let mut pos = 0;
    let bytes = text.as_bytes();

    while pos < text.len() {
        if let Some(caps) = INTERP_RE.captures(&text[pos..]) {
            let m0 = caps.get(0).unwrap();
            if m0.start() > 0 {
                // Unmatched '$' before match
                let skipped = &text[pos..pos + m0.start()];
                line += skipped.bytes().filter(|&b| b == b'\n').count();
                chunks.push(SplitChunk::Text(skipped.to_string()));
                pos += m0.start();
                continue;
            }
            pos += m0.end();

            if let Some(pre_m) = caps.get(1) {
                let pre = pre_m.as_str();
                line += pre.bytes().filter(|&b| b == b'\n').count();
                let unescaped = pre.replace(r"\$", "$");
                chunks.push(SplitChunk::Text(unescaped));
            } else if let Some(dir_m) = caps.get(2) {
                let dir = dir_m.as_str();
                let var = caps
                    .get(3)
                    .or_else(|| caps.get(4))
                    .map(|m| m.as_str())
                    .unwrap_or("");
                if !var.is_empty() {
                    line += dir.bytes().filter(|&b| b == b'\n').count();
                    let dir_tokens = tokenise_directive(var, anycase);
                    chunks.push(SplitChunk::Directive {
                        text: dir.to_string(),
                        line: line.to_string(),
                        tokens: dir_tokens,
                    });
                } else {
                    line += dir.bytes().filter(|&b| b == b'\n').count();
                    chunks.push(SplitChunk::Text(dir.to_string()));
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

static TOKEN_REST_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    RegexBuilder::new(
        r"\A(?:\s+|\#[^\n]*)|(?:\A(-?\d+(?:\.\d+)?))|(?:\A(\/?\w+(?:(?:\/|::?)\w*)+|\/\w+))|(?:\A(\w+))|(?:\A([(){}\[\]:;,\/\\]|[+\-*]|\$\{?|=>|[=!<>]?=|[!<>]|&&?|\|\|?|\.\.?|\S+))",
    )
    .multi_line(true)
    .dot_matches_new_line(true)
    .build()
    .unwrap()
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

        let slice = &text[pos..];
        if let Some(caps) = TOKEN_REST_RE.captures(slice) {
            let m0 = caps.get(0).unwrap();
            pos += m0.end();

            if let Some(num_m) = caps.get(1) {
                tokens.push(("NUMBER".to_string(), num_m.as_str().to_string()));
            } else if let Some(file_m) = caps.get(2) {
                tokens.push(("FILENAME".to_string(), file_m.as_str().to_string()));
            } else if let Some(ident_m) = caps.get(3) {
                let token = ident_m.as_str();
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
            } else if let Some(sym_m) = caps.get(4) {
                let token = sym_m.as_str();
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
            // else whitespace or comment: skip
        } else {
            // Advance by 1 char if nothing matched
            let ch_len = slice.chars().next().map(|c| c.len_utf8()).unwrap_or(1);
            pos += ch_len;
        }
    }

    tokens
}
