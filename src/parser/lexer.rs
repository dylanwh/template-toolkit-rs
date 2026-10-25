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
/// prefilters.  Tags that are plain literals, as `[% %]` and most named
/// styles are, skip the regex (and compiling it) for a `memmem` search.
pub struct Splitter {
    start: Tag,
    end: Tag,
    outline: Option<regex::Regex>,
}

/// A start or end tag pattern.
enum Tag {
    Literal(Box<memchr::memmem::Finder<'static>>),
    Regex(regex::Regex),
}

impl Tag {
    fn find_at(&self, hay: &str, at: usize) -> Option<(usize, usize)> {
        match self {
            Tag::Literal(f) => f
                .find(&hay.as_bytes()[at..])
                .map(|i| (at + i, at + i + f.needle().len())),
            Tag::Regex(re) => re.find_at(hay, at).map(|m| (m.start(), m.end())),
        }
    }
}

/// The text a tag pattern matches, if it is a plain literal under the
/// splitter's flags (`x`: unescaped whitespace is ignored): every regex
/// metacharacter escaped, and no escapes that stand for classes.
fn literal_pattern(pat: &str) -> Option<String> {
    let mut out = String::new();
    let mut chars = pat.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next()? {
                c if c.is_ascii_alphanumeric() || c.is_whitespace() => return None,
                c => out.push(c),
            },
            '.' | '^' | '$' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '#' => {
                return None
            }
            c if c.is_whitespace() => {}
            c => out.push(c),
        }
    }
    (!out.is_empty()).then_some(out)
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
        let tag = |pat: &str| match literal_pattern(pat) {
            Some(lit) => Ok(Tag::Literal(Box::new(
                memchr::memmem::Finder::new(lit.as_bytes()).into_owned(),
            ))),
            None => compile(format!("(?:{pat})"), false).map(Tag::Regex),
        };
        Ok(Self {
            start: tag(start)?,
            end: tag(end)?,
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
        let tag = self.start.find_at(hay, 0);
        let out = self.outline.as_ref().and_then(|re| re.find(hay));
        match (tag, out) {
            (Some((t_start, _)), Some(o)) if o.start() <= t_start => Some(outline(o)),
            (Some((t_start, t_end)), o) => match self.end.find_at(hay, t_end) {
                Some((e_start, e_end)) => Some(Split {
                    pre_end: t_start,
                    dir: t_end..e_start,
                    end: e_end,
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

// The chomp operations, as string operations rather than regexes (each
// would cost a regex compile on first use).  `char::is_whitespace` is the
// Unicode White_Space property, which is what the regex crate's `\s` is.

/// Horizontal whitespace: `[^\S\n]`.
fn is_hspace(c: char) -> bool {
    c.is_whitespace() && c != '\n'
}

/// Where `s/(\r?\n|^)[^\S\n]*\z//m` cuts `pre`: the trailing line, if it
/// is all horizontal whitespace, with the line break before it.
fn pre_chomp_all_cut(pre: &str) -> Option<usize> {
    let line_start = pre.rfind('\n').map_or(0, |i| i + 1);
    if !pre[line_start..].chars().all(is_hspace) {
        return None;
    }
    Some(match line_start.checked_sub(1) {
        Some(nl) if nl > 0 && pre.as_bytes()[nl - 1] == b'\r' => nl - 1,
        Some(nl) => nl,
        None => 0,
    })
}

/// The length of `\A[^\S\n]*\n` at the start of `s`, if it matches.
fn post_chomp_all_len(s: &str) -> Option<usize> {
    let ws = s.len() - s.trim_start_matches(is_hspace).len();
    s[ws..].starts_with('\n').then_some(ws + 1)
}

/// The length of `\A\s+` at the start of `s` (0 if none).
fn leading_ws_len(s: &str) -> usize {
    s.len() - s.trim_start().len()
}

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
                        if let Some(cut) = pre_chomp_all_cut(&pre) {
                            pre.truncate(cut);
                        }
                    }
                    CHOMP_COLLAPSE => {
                        let kept = pre.trim_end().len();
                        if kept < pre.len() {
                            pre.truncate(kept);
                            pre.push(' ');
                        }
                    }
                    CHOMP_GREEDY => {
                        pre.truncate(pre.trim_end().len());
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
                    if let Some(len) = post_chomp_all_len(remaining) {
                        pos += len;
                        postlines += 1;
                    }
                }
                CHOMP_COLLAPSE => {
                    let len = leading_ws_len(remaining);
                    if len > 0 {
                        postlines += remaining[..len].bytes().filter(|&b| b == b'\n').count();
                        pos += len;
                        lead_space = true;
                    }
                }
                CHOMP_GREEDY => {
                    let len = leading_ws_len(remaining);
                    postlines += remaining[..len].bytes().filter(|&b| b == b'\n').count();
                    pos += len;
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
const INTERP_SIMPLE: usize = 2;
#[cfg(test)]
static INTERP_RE: LazyLock<meta::Regex> =
    LazyLock::new(|| multi_regex(&[r"(?:\\.|[^\$])+", r"\$\{[^\}]*\}", r"\$[\w\.]+"]));

/// The leftmost match of those alternatives at or after `from`, as
/// `(start, pattern, end)`, without compiling a regex.  `\w` is the regex
/// crate's own Unicode word table.
fn interp_next(text: &str, from: usize) -> Option<(usize, usize, usize)> {
    let mut i = from;
    while i < text.len() {
        let rest = &text[i..];
        let Some(after) = rest.strip_prefix('$') else {
            // (?:\\.|[^\$])+
            let mut len = 0;
            let mut chars = rest.chars();
            while let Some(c) = chars.next() {
                if c == '$' {
                    break;
                }
                len += c.len_utf8();
                if c == '\\' {
                    len += chars.next().map_or(0, char::len_utf8);
                }
            }
            return Some((i, INTERP_TEXT, i + len));
        };
        // \$\{[^\}]*\}
        if let Some(close) = after.strip_prefix('{').and_then(|b| b.find('}')) {
            return Some((i, INTERP_BRACED, i + 2 + close + 1));
        }
        // \$[\w\.]+
        let name: usize = after
            .chars()
            .take_while(|&c| c == '.' || regex_syntax::is_word_character(c))
            .map(char::len_utf8)
            .sum();
        if name > 0 {
            return Some((i, INTERP_SIMPLE, i + 1 + name));
        }
        // A `$` that starts no match: the search moves on.
        i += 1;
    }
    None
}

pub fn interpolate_text(text: &str, mut line: usize, anycase: bool) -> Vec<SplitChunk> {
    let mut chunks = Vec::new();
    let mut pos = 0;
    let bytes = text.as_bytes();

    while pos < text.len() {
        if let Some((start, kind, end)) = interp_next(text, pos) {
            if start > pos {
                // Unmatched '$' before match
                let skipped = &text[pos..start];
                line += skipped.bytes().filter(|&b| b == b'\n').count();
                chunks.push(SplitChunk::Text(skipped.to_string()));
                pos = start;
                continue;
            }
            pos = end;
            let matched = &text[start..end];

            if kind == INTERP_TEXT {
                line += matched.bytes().filter(|&b| b == b'\n').count();
                chunks.push(SplitChunk::Text(matched.replace(r"\$", "$")));
            } else {
                let var = if kind == INTERP_BRACED {
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
const TOK_OTHER: usize = 4;
static TOKEN_REST_RE: LazyLock<meta::Regex> = LazyLock::new(|| {
    multi_regex(&[
        r"\s+|\#[^\n]*",
        r"-?\d+(?:\.\d+)?",
        r"\/?\w+(?:(?:\/|::?)\w*)+|\/\w+",
        r"\w+",
        r"[(){}\[\]:;,\/\\]|[+\-*]|\$\{?|=>|[=!<>]?=|[!<>]|&&?|\|\|?|\.\.?|\S+",
    ])
});

/// `\s` and `\w` restricted to ASCII.
fn is_ascii_space(b: u8) -> bool {
    matches!(b, b'\t' | b'\n' | 0x0B | 0x0C | b'\r' | b' ')
}

fn is_ascii_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// [`TOKEN_REST_RE`]'s anchored match at `pos`, as `(pattern, end)`, for
/// ASCII text: the same alternatives in the same order, with the same
/// leftmost-first, greedy choices.  Compiling the regex (Unicode `\w` and
/// `\d`) costs about a millisecond, which a process or Wasm instance that
/// parses a few templates and exits pays on every run.
///
/// The result is only valid if the bytes it depended on are ASCII: those up
/// to the match end plus two of lookahead (`1.5` needs `.` and a digit
/// after `1`).  [`fast_token`] checks that.
fn ascii_token(b: &[u8], pos: usize) -> (usize, usize) {
    let at = |i: usize| b.get(i).copied();
    let run = |mut i: usize, f: fn(u8) -> bool| {
        while at(i).is_some_and(f) {
            i += 1;
        }
        i
    };
    let c = b[pos];

    // \s+|\#[^\n]*
    if is_ascii_space(c) {
        return (TOK_SKIP, run(pos + 1, is_ascii_space));
    }
    if c == b'#' {
        return (
            TOK_SKIP,
            memchr::memchr(b'\n', &b[pos..]).map_or(b.len(), |i| pos + i),
        );
    }

    // -?\d+(?:\.\d+)?
    let digits = pos + usize::from(c == b'-');
    let int_end = run(digits, |d| d.is_ascii_digit());
    if int_end > digits {
        if at(int_end) == Some(b'.') && at(int_end + 1).is_some_and(|d| d.is_ascii_digit()) {
            return (TOK_NUMBER, run(int_end + 1, |d| d.is_ascii_digit()));
        }
        return (TOK_NUMBER, int_end);
    }

    // \/?\w+(?:(?:\/|::?)\w*)+|\/\w+
    let word = pos + usize::from(c == b'/');
    let word_end = run(word, is_ascii_word);
    if word_end > word {
        let mut i = word_end;
        loop {
            match at(i) {
                Some(b'/') => i += 1,
                Some(b':') => i += if at(i + 1) == Some(b':') { 2 } else { 1 },
                _ => break,
            }
            i = run(i, is_ascii_word);
        }
        if i > word_end {
            return (TOK_FILENAME, i);
        }
        if c == b'/' {
            return (TOK_FILENAME, word_end);
        }
        // \w+
        return (TOK_WORD, word_end);
    }

    // [(){}\[\]:;,\/\\]|[+\-*]|\$\{?|=>|[=!<>]?=|[!<>]|&&?|\|\|?|\.\.?|\S+
    let next = at(pos + 1);
    let len = match c {
        b'(' | b')' | b'{' | b'}' | b'[' | b']' | b':' | b';' | b',' | b'/' | b'\\' => 1,
        b'+' | b'-' | b'*' => 1,
        b'$' => 1 + usize::from(next == Some(b'{')),
        b'=' if next == Some(b'>') => 2,
        b'=' | b'!' | b'<' | b'>' if next == Some(b'=') => 2,
        b'=' | b'!' | b'<' | b'>' => 1,
        b'&' => 1 + usize::from(next == Some(b'&')),
        b'|' => 1 + usize::from(next == Some(b'|')),
        b'.' => 1 + usize::from(next == Some(b'.')),
        _ => return (TOK_OTHER, run(pos + 1, |x| !is_ascii_space(x))),
    };
    (TOK_OTHER, pos + len)
}

/// [`ascii_token`]'s answer, if the text it looked at was ASCII.
fn fast_token(b: &[u8], pos: usize) -> Option<(usize, usize)> {
    if !b[pos].is_ascii() {
        return None;
    }
    let (kind, end) = ascii_token(b, pos);
    // A comment runs to the newline whatever it contains.
    let looked_at = if kind == TOK_SKIP && b[pos] == b'#' {
        end
    } else {
        (end + 2).min(b.len())
    };
    b[pos..looked_at].is_ascii().then_some((kind, end))
}

static ANYCASE_BEFORE_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\A\s*[=\.]").unwrap());

/// Where Perl's quoted-string regex, `(["'])((?:\\\\|\\\2|.|\n)*?)\2`,
/// closes a string whose text starts at `from`.  Escapes are taken first,
/// but the lazy repeat backtracks: with no unescaped closing quote, an
/// escaped one (`\"`) still closes the string, leaving its `\` as text.
fn closing_quote(bytes: &[u8], from: usize, quote: u8) -> Option<usize> {
    let escape = |p: usize| {
        bytes[p] == b'\\' && p + 1 < bytes.len() && (bytes[p + 1] == b'\\' || bytes[p + 1] == quote)
    };

    // The regex's first try: every escape taken.
    let mut p = from;
    while p < bytes.len() {
        if bytes[p] == quote {
            return Some(p);
        }
        p += if escape(p) { 2 } else { 1 };
    }

    // That failed, so find the match backtracking reaches first: from each
    // position, the close it finds trying the alternatives in order.
    let mut close = vec![None; bytes.len() + 2];
    for p in (from..bytes.len()).rev() {
        close[p] = if bytes[p] == quote {
            Some(p)
        } else if escape(p) {
            close[p + 2].or(close[p + 1])
        } else {
            close[p + 1]
        };
    }
    close[from]
}

pub fn tokenise_directive(text: &str, anycase: bool) -> Vec<(String, String)> {
    let mut tokens: Vec<(String, String)> = Vec::new();
    let mut pos = 0;
    let bytes = text.as_bytes();

    while pos < text.len() {
        let b = bytes[pos];
        // Handle quoted strings: "..." or '...'
        if b == b'"' || b == b'\'' {
            let quote = b;
            if let Some(end_pos) = closing_quote(bytes, pos + 1, quote) {
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

        let matched = fast_token(bytes, pos).or_else(|| {
            let input = Input::new(text)
                .span(pos..text.len())
                .anchored(Anchored::Yes);
            TOKEN_REST_RE
                .search(&input)
                .map(|m| (m.pattern().as_usize(), m.end()))
        });
        if let Some((kind, end)) = matched {
            let token = &text[pos..end];
            pos = end;

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

#[cfg(test)]
mod fast_path_tests {
    //! Each hand-written scanner against the regex it replaces, on random
    //! strings over an alphabet of TT punctuation, digits, words and the
    //! Unicode that changes `\s`, `\w` and `\d` (no-break space, `é`, an
    //! Arabic-Indic digit).
    use super::*;

    const ALPHABET: &[&str] = &[
        " ", "\n", "\r", "\t", "a", "Z", "_", "x9", "0", "7", ".", "-", "+", "*", "/", ":", "::",
        "\\", "$", "{", "}", "(", ")", "[", "]", "=", "==", "=>", "!", "<", ">", "&", "|", "#",
        ";", ",", "'", "\"", "%", "é", "\u{663}", "\u{a0}", "ß", "@",
    ];

    fn strings(seed: u64, count: usize) -> Vec<String> {
        let mut state = seed;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as usize
        };
        (0..count)
            .map(|_| {
                (0..next() % 12)
                    .map(|_| ALPHABET[next() % ALPHABET.len()])
                    .collect()
            })
            .collect()
    }

    #[test]
    fn tokens_match_the_regex() {
        let mut fast = 0;
        for s in strings(1, 50_000) {
            let b = s.as_bytes();
            for pos in (0..s.len()).filter(|&i| s.is_char_boundary(i)) {
                let input = Input::new(&s).span(pos..s.len()).anchored(Anchored::Yes);
                let want = TOKEN_REST_RE
                    .search(&input)
                    .map(|m| (m.pattern().as_usize(), m.end()));
                if let Some(got) = fast_token(b, pos) {
                    fast += 1;
                    assert_eq!(Some(got), want, "{s:?} at {pos}");
                }
            }
        }
        assert!(
            fast > 100_000,
            "the fast path should handle most positions ({fast})"
        );
    }

    #[test]
    fn interpolation_matches_the_regex() {
        for s in strings(2, 50_000) {
            for pos in (0..s.len()).filter(|&i| s.is_char_boundary(i)) {
                let want = INTERP_RE
                    .search(&Input::new(&s).span(pos..s.len()))
                    .map(|m| (m.start(), m.pattern().as_usize(), m.end()));
                assert_eq!(interp_next(&s, pos), want, "{s:?} from {pos}");
            }
        }
    }

    #[test]
    fn closing_quotes_match_the_regex() {
        for (quote, pat) in [
            (b'"', r#"\A(?:\\\\|\\"|.)*?""#),
            (b'\'', r"\A(?:\\\\|\\'|.)*?'"),
        ] {
            let re = RegexBuilder::new(pat)
                .dot_matches_new_line(true)
                .build()
                .unwrap();
            for s in strings(4, 50_000) {
                assert_eq!(
                    closing_quote(s.as_bytes(), 0, quote),
                    re.find(&s).map(|m| m.end() - 1),
                    "{s:?}"
                );
            }
        }
        // No unescaped closing quote: the last escaped one closes the string.
        assert_eq!(closing_quote(br#"a\"b\" x"#, 0, b'"'), Some(5));
    }

    #[test]
    fn chomps_match_the_regexes() {
        let pre_all = RegexBuilder::new(r"(\r?\n|^)[^\S\n]*\z")
            .multi_line(true)
            .build()
            .unwrap();
        let post_all = regex::Regex::new(r"\A([^\S\n]*\n)").unwrap();
        let ws_start = regex::Regex::new(r"\A(\s+)").unwrap();
        let ws_end = regex::Regex::new(r"\s+\z").unwrap();
        for s in strings(3, 50_000) {
            assert_eq!(
                pre_chomp_all_cut(&s),
                pre_all.find(&s).map(|m| m.start()),
                "pre all {s:?}"
            );
            assert_eq!(
                post_chomp_all_len(&s),
                post_all.find(&s).map(|m| m.end()),
                "post all {s:?}"
            );
            assert_eq!(
                leading_ws_len(&s),
                ws_start.find(&s).map_or(0, |m| m.end()),
                "ws start {s:?}"
            );
            assert_eq!(
                s.trim_end().len(),
                ws_end.find(&s).map_or(s.len(), |m| m.start()),
                "ws end {s:?}"
            );
        }
    }

    #[test]
    fn literal_tags() {
        for (start, end, _) in ["default", "metatext", "html", "mason", "asp", "php", "star"]
            .iter()
            .filter_map(|name| get_tag_style(name))
        {
            for pat in [start, end] {
                let lit = literal_pattern(pat).unwrap_or_else(|| panic!("{pat} is a literal"));
                let re = RegexBuilder::new(&format!("^(?:{pat})$"))
                    .ignore_whitespace(true)
                    .build()
                    .unwrap();
                assert!(re.is_match(&lit), "{pat} matches {lit:?}");
            }
        }
        let (start, end, _) = get_tag_style("template1").unwrap();
        assert!(literal_pattern(start).is_none() && literal_pattern(end).is_none());
        for pat in [r"\d+", "a|b", "a.b", r"\ ", "(x)", "#", ""] {
            assert_eq!(literal_pattern(pat), None, "{pat}");
        }
        // TAGS directives escape their tags, so those are literals too.
        assert_eq!(literal_pattern(&regex::escape("<+")).as_deref(), Some("<+"));
    }
}
