use std::{collections::HashSet, sync::LazyLock};

use regex::RegexBuilder;
use regex_automata::{meta, util::syntax, Anchored, Input};

use crate::{
    constants::{CHOMP_ALL, CHOMP_COLLAPSE, CHOMP_GREEDY, CHOMP_NONE},
    parser::grammar_tables::lookup_lextable,
    regex_cache,
};

/// A byte range in the template source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start: start as u32,
            end: end as u32,
        }
    }

    /// An empty span at `at`.
    pub fn at(at: usize) -> Self {
        Self::new(at, at)
    }

    pub fn range(self) -> std::ops::Range<usize> {
        self.start as usize..self.end as usize
    }
}

/// Where each byte of the text being tokenised is in the source (and its
/// end, one past the last byte).
#[derive(Debug, Clone, Copy)]
pub enum SourceAt<'a> {
    /// Byte `i` is at `origin + i`, kept within `lo..=hi` (so a space a
    /// collapsing chomp put in front of the text stands where it starts).
    Shift { origin: usize, lo: usize, hi: usize },
    /// Byte `i` is at `table[i]`: text with escapes processed.
    Table(&'a [usize]),
}

impl SourceAt<'_> {
    /// The text is the source from `origin` on.
    pub fn offset(origin: usize) -> Self {
        SourceAt::Shift {
            origin,
            lo: 0,
            hi: usize::MAX,
        }
    }

    fn get(self, i: usize) -> usize {
        match self {
            SourceAt::Shift { origin, lo, hi } => (origin + i).clamp(lo, hi),
            SourceAt::Table(table) => table[i],
        }
    }

    /// The same for the text from byte `n` on.
    fn skip(self, n: usize) -> Self {
        match self {
            SourceAt::Shift { origin, lo, hi } => SourceAt::Shift {
                origin: origin + n,
                lo,
                hi,
            },
            SourceAt::Table(table) => SourceAt::Table(&table[n..]),
        }
    }
}

/// A token's or chunk's text: borrowed from the source, or owned where the
/// lexer had to change it (escapes, ANYCASE, chomping).  Two words.
pub type CowStr<'a> = beef::lean::Cow<'a, str>;

fn std_to_cow(s: std::borrow::Cow<'_, str>) -> CowStr<'_> {
    match s {
        std::borrow::Cow::Borrowed(s) => CowStr::borrowed(s),
        std::borrow::Cow::Owned(s) => CowStr::owned(s),
    }
}

/// An owned copy of `s`, for text that outlives what it borrowed from.
fn owned(s: CowStr<'_>) -> CowStr<'static> {
    CowStr::owned(s.into_owned())
}

#[derive(Debug, Clone)]
pub enum SplitChunk<'a> {
    Text(CowStr<'a>, Span),
    IText {
        text: CowStr<'a>,
        line: usize,
        span: Span,
        /// The source offset of `text`'s first byte, one before `span` when
        /// a collapsing chomp put a space in front of it.
        origin: usize,
    },
    Directive {
        text: CowStr<'a>,
        line: String,
        tokens: Vec<(&'static str, CowStr<'a>)>,
        /// One per token: where it is in the source.
        spans: Vec<Span>,
        /// The start tag (with any chomp flag), and the end tag (with its
        /// flag; for an outline directive, the newline ending it).
        open: Span,
        close: Span,
    },
    Token(&'static str, CowStr<'a>, Span),
    /// A comment or TAGS directive: nothing for the parser, but part of
    /// the source for highlighting.
    Inert {
        kind: InertKind,
        open: Span,
        body: Span,
        close: Span,
    },
}

impl SplitChunk<'_> {
    /// The chunk with nothing borrowed.
    pub fn into_owned(self) -> SplitChunk<'static> {
        match self {
            SplitChunk::Text(t, span) => SplitChunk::Text(owned(t), span),
            SplitChunk::IText {
                text,
                line,
                span,
                origin,
            } => SplitChunk::IText {
                text: owned(text),
                line,
                span,
                origin,
            },
            SplitChunk::Directive {
                text,
                line,
                tokens,
                spans,
                open,
                close,
            } => SplitChunk::Directive {
                text: owned(text),
                line,
                tokens: tokens.into_iter().map(|(t, v)| (t, owned(v))).collect(),
                spans,
                open,
                close,
            },
            SplitChunk::Token(t, v, span) => SplitChunk::Token(t, owned(v), span),
            SplitChunk::Inert {
                kind,
                open,
                body,
                close,
            } => SplitChunk::Inert {
                kind,
                open,
                body,
                close,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InertKind {
    Comment,
    Tags,
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

pub fn split_text<'a>(text: &'a str, style: &ParserStyle) -> Result<Vec<SplitChunk<'a>>, String> {
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
        // Source offsets of positions in `hay`; the leading space, if any,
        // stands where the text after it starts.
        let hay_start = pos;
        let origin = pos - usize::from(lead_space);
        let source = |i: usize| (origin + i).max(hay_start);
        let pre_span = Span::new(source(0), source(split.pre_end));
        // The text before the tag, and the directive, as slices of the
        // source: only a chomp's added space makes the text owned.
        let pre_body = &text[hay_start..(origin + split.pre_end).max(hay_start)];
        let mut pre: std::borrow::Cow<'a, str> = if lead_space && split.pre_end > 0 {
            std::borrow::Cow::Owned(format!(" {pre_body}"))
        } else {
            std::borrow::Cow::Borrowed(pre_body)
        };
        let mut dir: &'a str = &text[origin + split.dir.start..origin + split.dir.end];
        let dir_start = split.dir.start;
        let (tag_start, dir_end, split_end) = (
            source(split.pre_end),
            source(split.dir.end),
            source(split.end),
        );
        pos += split.end - usize::from(lead_space);
        lead_space = false;

        let mut prelines = pre.bytes().filter(|&b| b == b'\n').count();
        let mut dirlines = dir.bytes().filter(|&b| b == b'\n').count();
        let mut postlines = 0;
        // Bytes cut from the front of the directive: a chomp flag and
        // whitespace.
        let mut lead_cut = 0;
        // Of which the chomp flag.
        let mut flag_len = 0;
        let comment = dir.starts_with('#');

        if comment {
            // Comment out entire directive except for any end chomp flag
            dir = match dir.chars().last() {
                Some(last_ch) if chomp_char_to_mode(last_ch).is_some() => {
                    &dir[dir.len() - last_ch.len_utf8()..]
                }
                _ => "",
            };
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
            let ws_len = dir[ws_start..].len() - dir[ws_start..].trim_start().len();
            if ws_len > 0 {
                let linecount = dir[ws_start..ws_start + ws_len]
                    .bytes()
                    .filter(|&b| b == b'\n')
                    .count();
                prelines += linecount;
                dirlines = dirlines.saturating_sub(linecount);
            }
            dir = &dir[ws_start + ws_len..];
            lead_cut = ws_start + ws_len;
            flag_len = ws_start;

            let chomp = pre_flag.unwrap_or(prechomp);
            if chomp != CHOMP_NONE && !pre.is_empty() {
                match chomp {
                    CHOMP_ALL => {
                        if let Some(cut) = pre_chomp_all_cut(&pre) {
                            truncate(&mut pre, cut);
                        }
                    }
                    CHOMP_COLLAPSE => {
                        let kept = pre.trim_end().len();
                        if kept < pre.len() {
                            truncate(&mut pre, kept);
                            pre.to_mut().push(' ');
                        }
                    }
                    CHOMP_GREEDY => {
                        let kept = pre.trim_end().len();
                        truncate(&mut pre, kept);
                    }
                    _ => {}
                }
            }
        }

        // POST_CHOMP: s/\s*($CHOMP_FLAGS)?\s*$//so
        let trimmed_end = dir.trim_end();
        let mut post_flag = None;
        let mut close_start = dir_end;
        let new_dir_len = if let Some(last_ch) = trimmed_end.chars().last() {
            if let Some(mode) = chomp_char_to_mode(last_ch) {
                post_flag = Some(mode);
                let flag_at = trimmed_end.len() - last_ch.len_utf8();
                if !comment {
                    close_start = source(dir_start + lead_cut + flag_at);
                }
                trimmed_end[..flag_at].trim_end().len()
            } else {
                trimmed_end.len()
            }
        } else {
            0
        };
        dir = &dir[..new_dir_len];
        let open = Span::new(tag_start, source(dir_start + flag_len).min(close_start));
        let close = Span::new(close_start, split_end);
        let body_start = source(dir_start + lead_cut).min(close_start);
        let body = Span::new(body_start, (body_start + new_dir_len).min(close_start));

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
            let pre = std_to_cow(pre);
            if interp {
                tokens.push(SplitChunk::IText {
                    text: pre,
                    line,
                    span: pre_span,
                    origin,
                });
            } else {
                tokens.push(SplitChunk::Text(pre, pre_span));
            }
        }
        line += prelines;

        if comment {
            let body = Span::new(open.end as usize, close.start as usize);
            tokens.push(SplitChunk::Inert {
                kind: InertKind::Comment,
                open,
                body,
                close,
            });
        } else if !dir.is_empty() {
            // Check TAGS directive
            let is_tags = if anycase {
                dir.len() > 4
                    && dir[..4].eq_ignore_ascii_case("TAGS")
                    && dir[4..].starts_with(char::is_whitespace)
            } else {
                dir.starts_with("TAGS") && dir[4..].starts_with(char::is_whitespace)
            };

            if is_tags {
                tokens.push(SplitChunk::Inert {
                    kind: InertKind::Tags,
                    open,
                    body,
                    close,
                });
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
                let mut dir_tokens = Vec::new();
                let mut spans = Vec::new();
                let base = body.start as usize;
                tokenise_directive_at(
                    dir,
                    anycase,
                    SourceAt::offset(base),
                    &mut dir_tokens,
                    &mut spans,
                );
                tokens.push(SplitChunk::Directive {
                    text: CowStr::borrowed(dir),
                    line: line_str,
                    tokens: dir_tokens,
                    spans,
                    open,
                    close,
                });
            }
        }

        line += dirlines + postlines;
    }

    let remaining = if lead_space {
        CowStr::owned(format!(" {}", &text[pos..]))
    } else {
        CowStr::borrowed(&text[pos..])
    };
    if !remaining.is_empty() {
        let span = Span::new(pos, text.len());
        if interp {
            tokens.push(SplitChunk::IText {
                text: remaining,
                line,
                span,
                origin: pos - usize::from(lead_space),
            });
        } else {
            tokens.push(SplitChunk::Text(remaining, span));
        }
    }

    Ok(tokens)
}

/// `s.truncate(len)`, keeping a borrowed `s` borrowed.
fn truncate(s: &mut std::borrow::Cow<'_, str>, len: usize) {
    match s {
        std::borrow::Cow::Borrowed(b) => *b = &b[..len],
        std::borrow::Cow::Owned(o) => o.truncate(len),
    }
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

pub fn interpolate_text(text: &str, line: usize, anycase: bool) -> Vec<SplitChunk<'_>> {
    interpolate_text_at(text, line, anycase, SourceAt::offset(0))
}

/// `interpolate_text`, with `at` giving the source offset of each byte of
/// `text` (and of its end) for the chunks' spans.
pub fn interpolate_text_at<'t>(
    text: &'t str,
    mut line: usize,
    anycase: bool,
    at: SourceAt,
) -> Vec<SplitChunk<'t>> {
    let mut chunks = Vec::new();
    let mut pos = 0;
    let bytes = text.as_bytes();
    let span = |start: usize, end: usize| Span::new(at.get(start), at.get(end));

    while pos < text.len() {
        if let Some((start, kind, end)) = interp_next(text, pos) {
            if start > pos {
                // Unmatched '$' before match
                let skipped = &text[pos..start];
                line += skipped.bytes().filter(|&b| b == b'\n').count();
                chunks.push(SplitChunk::Text(
                    CowStr::borrowed(skipped),
                    span(pos, start),
                ));
                pos = start;
                continue;
            }
            pos = end;
            let matched = &text[start..end];

            if kind == INTERP_TEXT {
                line += matched.bytes().filter(|&b| b == b'\n').count();
                let unescaped = if matched.contains(r"\$") {
                    CowStr::owned(matched.replace(r"\$", "$"))
                } else {
                    CowStr::borrowed(matched)
                };
                chunks.push(SplitChunk::Text(unescaped, span(start, end)));
            } else {
                let (var_start, var_end) = if kind == INTERP_BRACED {
                    (start + 2, end - 1)
                } else {
                    (start + 1, end)
                };
                let var = &text[var_start..var_end];
                line += matched.bytes().filter(|&b| b == b'\n').count();
                if !var.is_empty() {
                    let mut dir_tokens = Vec::new();
                    let mut spans = Vec::new();
                    tokenise_directive_at(
                        var,
                        anycase,
                        at.skip(var_start),
                        &mut dir_tokens,
                        &mut spans,
                    );
                    chunks.push(SplitChunk::Directive {
                        text: CowStr::borrowed(matched),
                        line: line.to_string(),
                        tokens: dir_tokens,
                        spans,
                        open: span(start, var_start),
                        close: span(var_end, end),
                    });
                } else {
                    chunks.push(SplitChunk::Text(
                        CowStr::borrowed(matched),
                        span(start, end),
                    ));
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
            chunks.push(SplitChunk::Text(
                CowStr::borrowed(slice),
                span(pos, pos + ch_len),
            ));
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

pub fn tokenise_directive(text: &str, anycase: bool) -> Vec<(&'static str, String)> {
    let mut tokens = Vec::new();
    tokenise_directive_at(
        text,
        anycase,
        SourceAt::offset(0),
        &mut tokens,
        &mut Vec::new(),
    );
    tokens
        .into_iter()
        .map(|(t, v)| (t, v.into_owned()))
        .collect()
}

/// `tokenise_directive`, appending to `tokens` and, for each token, its
/// span to `spans`; `at` gives the source offset of each byte of `text`.
pub fn tokenise_directive_at<'t>(
    text: &'t str,
    anycase: bool,
    at: SourceAt,
    tokens: &mut Vec<(&'static str, CowStr<'t>)>,
    spans: &mut Vec<Span>,
) {
    let mut pos = 0;
    let bytes = text.as_bytes();
    let span = |start: usize, end: usize| Span::new(at.get(start), at.get(end));

    while pos < text.len() {
        let b = bytes[pos];
        // Handle quoted strings: "..." or '...'
        if b == b'"' || b == b'\'' {
            let quote = b;
            if let Some(end_pos) = closing_quote(bytes, pos + 1, quote) {
                let inner_start = pos + 1;
                let inner = &text[inner_start..end_pos];
                let whole = span(pos, end_pos + 1);
                pos = end_pos + 1;
                if quote == b'"' {
                    if inner.contains('$') || inner.contains('\\') {
                        // Process escapes:
                        // s/\\([^\$nrt])/$1/g;
                        // s/\\([nrt])/$QUOTED_ESCAPES->{ $1 }/ge;
                        // `source` maps each byte of the result to where it
                        // came from (an escape, to its backslash).
                        let mut processed = String::with_capacity(inner.len());
                        let mut source: Vec<usize> = Vec::with_capacity(inner.len() + 1);
                        let mut push = |s: &str, from: usize, processed: &mut String| {
                            processed.push_str(s);
                            source.extend(std::iter::repeat_n(at.get(inner_start + from), s.len()));
                        };
                        let mut chars = inner.char_indices().peekable();
                        while let Some((i, c)) = chars.next() {
                            if c == '\\' {
                                if let Some(&(_, next_c)) = chars.peek() {
                                    chars.next();
                                    match next_c {
                                        '$' => push("\\$", i, &mut processed),
                                        'n' => push("\n", i, &mut processed),
                                        'r' => push("\r", i, &mut processed),
                                        't' => push("\t", i, &mut processed),
                                        other => {
                                            push(other.encode_utf8(&mut [0; 4]), i, &mut processed)
                                        }
                                    }
                                } else {
                                    push("\\", i, &mut processed);
                                }
                            } else {
                                push(c.encode_utf8(&mut [0; 4]), i, &mut processed);
                            }
                        }
                        source.push(at.get(end_pos));

                        tokens.push(("\"", CowStr::borrowed("\"")));
                        spans.push(Span::new(whole.start as usize, whole.start as usize + 1));
                        let interp_chunks =
                            interpolate_text_at(&processed, 1, anycase, SourceAt::Table(&source));
                        for chunk in interp_chunks {
                            match chunk {
                                // `processed` is a local, so what borrows from it is copied.
                                SplitChunk::Text(t, s) => {
                                    tokens.push(("TEXT", owned(t)));
                                    spans.push(s);
                                }
                                SplitChunk::Directive {
                                    tokens: sub_tokens,
                                    spans: sub_spans,
                                    close,
                                    ..
                                } => {
                                    tokens
                                        .extend(sub_tokens.into_iter().map(|(t, v)| (t, owned(v))));
                                    spans.extend(sub_spans);
                                    tokens.push((";", CowStr::borrowed(";")));
                                    spans.push(Span::at(close.start as usize));
                                }
                                _ => {}
                            }
                        }
                        tokens.push(("\"", CowStr::borrowed("\"")));
                        spans.push(Span::new(whole.end as usize - 1, whole.end as usize));
                    } else {
                        let escaped = inner.replace('\'', r"\'");
                        tokens.push(("LITERAL", CowStr::owned(format!("'{}'", escaped))));
                        spans.push(whole);
                    }
                } else {
                    // `'inner'` is the source text itself.
                    tokens.push((
                        "LITERAL",
                        CowStr::borrowed(&text[inner_start - 1..end_pos + 1]),
                    ));
                    spans.push(whole);
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
            if kind != TOK_SKIP {
                spans.push(span(pos, end));
            }
            pos = end;

            if kind == TOK_SKIP {
                // whitespace or comment
            } else if kind == TOK_NUMBER {
                tokens.push(("NUMBER", CowStr::borrowed(token)));
            } else if kind == TOK_FILENAME {
                tokens.push(("FILENAME", CowStr::borrowed(token)));
            } else if kind == TOK_WORD {
                // Under ANYCASE a word is looked up in upper case, unless
                // what is around it makes it a name.
                let folded = anycase && !token.bytes().all(|b| !b.is_ascii_lowercase()) && {
                    let prev_blocks_fold = tokens
                        .last()
                        .is_some_and(|(t, _)| ANYCASE_AFTER.contains(t));
                    !(prev_blocks_fold || ANYCASE_BEFORE_RE.is_match(&text[pos..]))
                };
                let upper = folded.then(|| token.to_ascii_uppercase());
                match lookup_lextable(upper.as_deref().unwrap_or(token)) {
                    Some(lex_type) => tokens.push((
                        lex_type,
                        upper.map_or(CowStr::borrowed(token), CowStr::owned),
                    )),
                    None => tokens.push(("IDENT", CowStr::borrowed(token))),
                }
            } else {
                let lex_type = if anycase {
                    lookup_lextable(&token.to_ascii_uppercase())
                } else {
                    lookup_lextable(token)
                };
                tokens.push((lex_type.unwrap_or("UNQUOTED"), CowStr::borrowed(token)));
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
