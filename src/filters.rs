use std::{
    cell::RefCell,
    collections::HashMap,
    fs,
    io::Write,
    path::PathBuf,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    error::{ControlFlow, TemplateException},
    value::Value,
};

static URI_RFC2732: AtomicBool = AtomicBool::new(false);

pub fn use_rfc2732() {
    URI_RFC2732.store(true, Ordering::SeqCst);
}

pub fn use_rfc3986() {
    URI_RFC2732.store(false, Ordering::SeqCst);
}

pub type FilterFn = Rc<dyn Fn(&str) -> Result<String, ControlFlow>>;

/// Dynamic filter factory signature: receives `args: &[Value]` and returns a `FilterFn` or error.
pub type DynamicFilterFactory = Rc<dyn Fn(&[Value]) -> Result<FilterFactoryOutcome, ControlFlow>>;

pub enum FilterFactoryOutcome {
    Ok(FilterFn),
    InvalidReturn,
    Error(TemplateException),
}

#[derive(Clone)]
pub enum FilterDef {
    Static(FilterFn),
    Dynamic(DynamicFilterFactory),
    InvalidEntry,
}

#[derive(Clone)]
pub struct Filters {
    pub local_filters: HashMap<String, FilterDef>,
    pub tolerant: bool,
    pub output_path: Option<PathBuf>,
    pub stderr_sink: Option<Rc<RefCell<String>>>,
}

impl Default for Filters {
    fn default() -> Self {
        Self::new()
    }
}

impl Filters {
    pub fn new() -> Self {
        Self {
            local_filters: HashMap::new(),
            tolerant: false,
            output_path: None,
            stderr_sink: None,
        }
    }

    pub fn store(&mut self, name: impl Into<String>, def: FilterDef) {
        self.local_filters.insert(name.into(), def);
    }

    pub fn fetch(&self, name: &str, args: &[Value]) -> Result<Option<FilterFn>, TemplateException> {
        let def_opt = self
            .local_filters
            .get(name)
            .cloned()
            .or_else(|| self.builtin_filter(name));

        let Some(def) = def_opt else {
            return Ok(None); // STATUS_DECLINED
        };

        let err_or_fn: Result<FilterFn, TemplateException> = match def {
            FilterDef::Static(f) => Ok(f),
            FilterDef::InvalidEntry => Err(TemplateException::new(
                "filter",
                format!("invalid FILTER entry for '{}' (not a CODE ref)", name),
            )),
            FilterDef::Dynamic(factory) => match factory(args) {
                Ok(FilterFactoryOutcome::Ok(f)) => Ok(f),
                Ok(FilterFactoryOutcome::InvalidReturn) => Err(TemplateException::new(
                    "filter",
                    format!("invalid FILTER for '{}' (not a CODE ref)", name),
                )),
                Ok(FilterFactoryOutcome::Error(ex)) => Err(ex),
                Err(ControlFlow::Exception(mut ex)) => {
                    if ex.type_.is_empty() || ex.type_ == "undef" {
                        ex.type_ = "filter".to_string();
                    }
                    Err(ex)
                }
                Err(_) => Err(TemplateException::new("filter", "filter factory aborted")),
            },
        };

        match err_or_fn {
            Ok(f) => Ok(Some(f)),
            Err(ex) => {
                if self.tolerant {
                    Ok(None)
                } else {
                    Err(ex)
                }
            }
        }
    }

    fn builtin_filter(&self, name: &str) -> Option<FilterDef> {
        match name {
            "html" => Some(FilterDef::Static(Rc::new(|text| Ok(html_filter(text))))),
            "xml" => Some(FilterDef::Static(Rc::new(|text| Ok(xml_filter(text))))),
            "html_para" => Some(FilterDef::Static(Rc::new(|text| Ok(html_paragraph(text))))),
            "html_break" | "html_para_break" => {
                Some(FilterDef::Static(Rc::new(|text| Ok(html_para_break(text)))))
            }
            "html_line_break" => Some(FilterDef::Static(Rc::new(|text| Ok(html_line_break(text))))),
            "uri" => Some(FilterDef::Static(Rc::new(|text| Ok(uri_filter(text))))),
            "url" => Some(FilterDef::Static(Rc::new(|text| Ok(url_filter(text))))),
            "upper" => Some(FilterDef::Static(Rc::new(|text| Ok(text.to_uppercase())))),
            "lower" => Some(FilterDef::Static(Rc::new(|text| Ok(text.to_lowercase())))),
            "ucfirst" => Some(FilterDef::Static(Rc::new(|text| {
                let mut chars = text.chars();
                match chars.next() {
                    None => Ok(String::new()),
                    Some(first) => Ok(first.to_uppercase().collect::<String>() + chars.as_str()),
                }
            }))),
            "lcfirst" => Some(FilterDef::Static(Rc::new(|text| {
                let mut chars = text.chars();
                match chars.next() {
                    None => Ok(String::new()),
                    Some(first) => Ok(first.to_lowercase().collect::<String>() + chars.as_str()),
                }
            }))),
            "stderr" => {
                let sink = self.stderr_sink.clone();
                Some(FilterDef::Static(Rc::new(move |text| {
                    if let Some(ref s) = sink {
                        s.borrow_mut().push_str(text);
                    } else {
                        eprint!("{}", text);
                    }
                    Ok(String::new())
                })))
            }
            "trim" => Some(FilterDef::Static(Rc::new(|text| {
                Ok(text.trim().to_string())
            }))),
            "null" => Some(FilterDef::Static(Rc::new(|_| Ok(String::new())))),
            "collapse" => Some(FilterDef::Static(Rc::new(|text| {
                let trimmed = text.trim();
                let mut out = String::with_capacity(trimmed.len());
                let mut in_ws = false;
                for c in trimmed.chars() {
                    if c.is_whitespace() {
                        if !in_ws {
                            out.push(' ');
                            in_ws = true;
                        }
                    } else {
                        in_ws = false;
                        out.push(c);
                    }
                }
                Ok(out)
            }))),
            "html_entity" => Some(FilterDef::Dynamic(Rc::new(|_args| {
                Ok(FilterFactoryOutcome::Ok(Rc::new(|text| {
                    Ok(html_entity_filter(text))
                })))
            }))),
            "indent" => Some(FilterDef::Dynamic(Rc::new(|args| {
                let pad = match args.first() {
                    None | Some(Value::Undefined) => "    ".to_string(),
                    Some(v) => {
                        let s = v.to_tt_string();
                        if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) {
                            let n = s.parse::<usize>().unwrap_or(4);
                            " ".repeat(n)
                        } else {
                            s
                        }
                    }
                };
                Ok(FilterFactoryOutcome::Ok(Rc::new(move |text| {
                    if text.is_empty() {
                        return Ok(String::new());
                    }
                    // Perl: $text =~ s/^/$pad/mg;
                    // Note: in Perl with /mg, ^ matches at start of string and after every \n
                    // EXCEPT at the very end of the string right after a trailing \n!
                    let mut out = String::new();
                    let parts: Vec<&str> = text.split('\n').collect();
                    for (i, part) in parts.iter().enumerate() {
                        if i > 0 {
                            out.push('\n');
                        }
                        if i == parts.len() - 1 && part.is_empty() && i > 0 {
                            // Do not indent empty trailing position after final \n
                        } else {
                            out.push_str(&pad);
                            out.push_str(part);
                        }
                    }
                    Ok(out)
                })))
            }))),
            "format" => Some(FilterDef::Dynamic(Rc::new(|args| {
                let fmt_str = args
                    .first()
                    .filter(|v| v.is_defined())
                    .map(|v| v.to_tt_string())
                    .unwrap_or_else(|| "%s".to_string());
                Ok(FilterFactoryOutcome::Ok(Rc::new(move |text| {
                    let lines = perl_split_lines(text);
                    let formatted: Vec<String> = lines
                        .into_iter()
                        .map(|line| perl_sprintf(&fmt_str, &[Value::Str(line.to_string())]))
                        .collect();
                    Ok(formatted.join("\n"))
                })))
            }))),
            "repeat" => Some(FilterDef::Dynamic(Rc::new(|args| {
                let iter = args
                    .first()
                    .filter(|v| v.is_defined() && !v.to_tt_string().is_empty())
                    .map(|v| v.to_int().max(0) as usize)
                    .unwrap_or(1);
                Ok(FilterFactoryOutcome::Ok(Rc::new(move |text| {
                    Ok(text.repeat(iter))
                })))
            }))),
            "replace" => Some(FilterDef::Dynamic(Rc::new(|args| {
                let search = args
                    .first()
                    .filter(|v| v.is_defined())
                    .map(|v| v.to_tt_string())
                    .unwrap_or_default();
                let replace = args
                    .get(1)
                    .filter(|v| v.is_defined())
                    .map(|v| v.to_tt_string())
                    .unwrap_or_default();
                Ok(FilterFactoryOutcome::Ok(Rc::new(move |text| {
                    if let Ok(re) = regex::Regex::new(&search) {
                        Ok(re.replace_all(text, replace.as_str()).into_owned())
                    } else if let Ok(fre) = fancy_regex::Regex::new(&search) {
                        Ok(fre.replace_all(text, replace.as_str()).into_owned())
                    } else {
                        Ok(text.replace(&search, &replace))
                    }
                })))
            }))),
            "remove" => Some(FilterDef::Dynamic(Rc::new(|args| {
                let search = args
                    .first()
                    .filter(|v| v.is_defined())
                    .map(|v| v.to_tt_string())
                    .unwrap_or_default();
                Ok(FilterFactoryOutcome::Ok(Rc::new(move |text| {
                    if let Ok(re) = regex::Regex::new(&search) {
                        Ok(re.replace_all(text, "").into_owned())
                    } else if let Ok(fre) = fancy_regex::Regex::new(&search) {
                        Ok(fre.replace_all(text, "").into_owned())
                    } else {
                        Ok(text.replace(&search, ""))
                    }
                })))
            }))),
            "truncate" => Some(FilterDef::Dynamic(Rc::new(|args| {
                let len = args
                    .first()
                    .filter(|v| v.is_defined())
                    .map(|v| v.to_int().max(0) as usize)
                    .unwrap_or(32);
                let mut addon = args
                    .get(1)
                    .filter(|v| v.is_defined())
                    .map(|v| v.to_tt_string())
                    .unwrap_or_else(|| "...".to_string());
                let addon_chars: Vec<char> = addon.chars().collect();
                let lchar = if len < addon_chars.len() {
                    addon = addon_chars[..len].iter().collect();
                    len
                } else {
                    addon_chars.len()
                };
                Ok(FilterFactoryOutcome::Ok(Rc::new(move |text| {
                    let chars: Vec<char> = text.chars().collect();
                    if chars.len() <= len {
                        Ok(text.to_string())
                    } else {
                        let prefix: String = chars[..len - lchar].iter().collect();
                        Ok(format!("{}{}", prefix, addon))
                    }
                })))
            }))),
            "perl" | "evalperl" => Some(FilterDef::Dynamic(Rc::new(|_args| {
                Ok(FilterFactoryOutcome::Error(TemplateException::new(
                    "perl",
                    "EVAL_PERL is not set",
                )))
            }))),
            "redirect" | "file" => {
                let outpath = self.output_path.clone();
                Some(FilterDef::Dynamic(Rc::new(move |args| {
                    let Some(ref base_dir) = outpath else {
                        return Ok(FilterFactoryOutcome::Error(TemplateException::new(
                            "redirect",
                            "OUTPUT_PATH is not set",
                        )));
                    };
                    let file = args.first().map(|v| v.to_tt_string()).unwrap_or_default();
                    if file.starts_with("../") || file.contains("/../") {
                        return Ok(FilterFactoryOutcome::Error(TemplateException::new(
                            "redirect",
                            format!("relative filenames are not supported: {}", file),
                        )));
                    }
                    let full_path = base_dir.join(&file);
                    Ok(FilterFactoryOutcome::Ok(Rc::new(move |text| {
                        if let Some(parent) = full_path.parent() {
                            if let Err(e) = fs::create_dir_all(parent) {
                                return Err(ControlFlow::Exception(TemplateException::new(
                                    "redirect",
                                    e.to_string(),
                                )));
                            }
                        }
                        if let Err(e) = fs::write(&full_path, text) {
                            return Err(ControlFlow::Exception(TemplateException::new(
                                "redirect",
                                e.to_string(),
                            )));
                        }
                        Ok(String::new())
                    })))
                })))
            }
            "stdout" => Some(FilterDef::Dynamic(Rc::new(|_args| {
                Ok(FilterFactoryOutcome::Ok(Rc::new(|text| {
                    let _ = std::io::stdout().write_all(text.as_bytes());
                    Ok(String::new())
                })))
            }))),
            _ => None,
        }
    }
}

pub fn html_filter(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn xml_filter(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn html_entity_filter(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            'é' => out.push_str("&eacute;"),
            c if (c as u32) > 127 => {
                out.push_str(&format!("&#{};", c as u32));
            }
            other => out.push(other),
        }
    }
    out
}

pub fn html_paragraph(text: &str) -> String {
    let re = regex::Regex::new(r"(?:\r?\n){2,}").unwrap();
    // Perl split(/(?:\r?\n){2,}/, $text) drops trailing empty fields
    let mut parts: Vec<&str> = re.split(text).collect();
    while parts.len() > 1 && parts.last() == Some(&"") {
        parts.pop();
    }
    if text.is_empty() {
        return "<p>\n</p>\n".to_string();
    }
    format!("<p>\n{}</p>\n", parts.join("\n</p>\n\n<p>\n"))
}

pub fn html_para_break(text: &str) -> String {
    let re = regex::Regex::new(r"(\r?\n){2,}").unwrap();
    re.replace_all(text, "$1<br />$1<br />$1").into_owned()
}

pub fn html_line_break(text: &str) -> String {
    let re = regex::Regex::new(r"(\r?\n)").unwrap();
    re.replace_all(text, "<br />$1").into_owned()
}

fn is_rfc_safe(b: u8, rfc2732: bool, is_url: bool) -> bool {
    if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
        return true;
    }
    if rfc2732 && matches!(b, b'!' | b'*' | b'\'' | b'(' | b')') {
        return true;
    }
    if is_url
        && matches!(
            b,
            b';' | b'/' | b'?' | b':' | b'@' | b'&' | b'=' | b'+' | b'$' | b','
        )
    {
        return true;
    }
    false
}

pub fn uri_filter(text: &str) -> String {
    let rfc2732 = URI_RFC2732.load(Ordering::SeqCst);
    let mut out = String::with_capacity(text.len());
    for &b in text.as_bytes() {
        if is_rfc_safe(b, rfc2732, false) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

pub fn url_filter(text: &str) -> String {
    let rfc2732 = URI_RFC2732.load(Ordering::SeqCst);
    let mut out = String::with_capacity(text.len());
    for &b in text.as_bytes() {
        if is_rfc_safe(b, rfc2732, true) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

pub fn perl_split_lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut parts: Vec<&str> = text.split('\n').collect();
    while parts.last() == Some(&"") {
        parts.pop();
    }
    parts
}

/// Format a string using Perl/C `sprintf` specifiers (`%s`, `%-10s`, `%02d`, `%.2f`, etc.).
pub fn perl_sprintf(fmt_str: &str, args: &[Value]) -> String {
    let mut out = String::with_capacity(fmt_str.len());
    let mut chars = fmt_str.chars().peekable();
    let mut arg_idx = 0;

    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        if chars.peek() == Some(&'%') {
            out.push('%');
            chars.next();
            continue;
        }

        // Parse flags: -, +, 0, space, #
        let mut left_align = false;
        let mut plus_sign = false;
        let mut zero_pad = false;
        let mut space_sign = false;
        while let Some(&fc) = chars.peek() {
            match fc {
                '-' => {
                    left_align = true;
                    chars.next();
                }
                '+' => {
                    plus_sign = true;
                    chars.next();
                }
                '0' => {
                    zero_pad = true;
                    chars.next();
                }
                ' ' => {
                    space_sign = true;
                    chars.next();
                }
                '#' => {
                    chars.next();
                }
                _ => break,
            }
        }

        // Parse width (or '*' dynamic width)
        let width: Option<usize> = if chars.peek() == Some(&'*') {
            chars.next();
            let w_val = args.get(arg_idx).cloned().unwrap_or(Value::Undefined);
            arg_idx += 1;
            let w_int = w_val.to_int();
            if w_int < 0 {
                left_align = true;
            }
            Some(w_int.unsigned_abs() as usize)
        } else {
            let mut width_str = String::new();
            while let Some(&wc) = chars.peek() {
                if wc.is_ascii_digit() {
                    width_str.push(wc);
                    chars.next();
                } else {
                    break;
                }
            }
            width_str.parse().ok()
        };

        // Parse precision (or '.*' dynamic precision)
        let mut prec: Option<usize> = None;
        if chars.peek() == Some(&'.') {
            chars.next();
            if chars.peek() == Some(&'*') {
                chars.next();
                let p_val = args.get(arg_idx).cloned().unwrap_or(Value::Undefined);
                arg_idx += 1;
                prec = Some(p_val.to_int().max(0) as usize);
            } else {
                let mut p_str = String::new();
                while let Some(&pc) = chars.peek() {
                    if pc.is_ascii_digit() {
                        p_str.push(pc);
                        chars.next();
                    } else {
                        break;
                    }
                }
                prec = Some(p_str.parse().unwrap_or(0));
            }
        }

        let Some(spec) = chars.next() else {
            break;
        };

        let val = args.get(arg_idx).cloned().unwrap_or(Value::Undefined);
        arg_idx += 1;

        let formatted = match spec {
            's' => {
                let mut s = val.to_tt_string();
                if let Some(p) = prec {
                    s = s.chars().take(p).collect();
                }
                pad_string(s, width, left_align, false)
            }
            'd' | 'i' => {
                let n = val.to_int();
                let sign = if n < 0 {
                    "-"
                } else if plus_sign {
                    "+"
                } else if space_sign {
                    " "
                } else {
                    ""
                };
                let mut digits = n.unsigned_abs().to_string();
                if let Some(p) = prec {
                    while digits.len() < p {
                        digits.insert(0, '0');
                    }
                }
                if zero_pad && !left_align && prec.is_none() {
                    if let Some(w) = width {
                        let target = w.saturating_sub(sign.len());
                        while digits.len() < target {
                            digits.insert(0, '0');
                        }
                    }
                }
                let combined = format!("{}{}", sign, digits);
                pad_string(combined, width, left_align, false)
            }
            'u' => {
                let n = val.to_int().max(0) as u64;
                let s = n.to_string();
                pad_string(s, width, left_align, zero_pad && !left_align)
            }
            'x' | 'X' => {
                let n = val.to_int() as u64;
                let s = if spec == 'x' {
                    format!("{:x}", n)
                } else {
                    format!("{:X}", n)
                };
                pad_string(s, width, left_align, zero_pad && !left_align)
            }
            'o' => {
                let n = val.to_int() as u64;
                let s = format!("{:o}", n);
                pad_string(s, width, left_align, zero_pad && !left_align)
            }
            'f' | 'F' => {
                let f = val.to_number();
                let p = prec.unwrap_or(6);
                let s = format!("{:.*}", p, f);
                pad_string(s, width, left_align, zero_pad && !left_align)
            }
            'c' => {
                let code = val.to_int() as u8 as char;
                pad_string(code.to_string(), width, left_align, false)
            }
            other => other.to_string(),
        };

        out.push_str(&formatted);
    }

    out
}

fn pad_string(s: String, width: Option<usize>, left_align: bool, zero_pad: bool) -> String {
    let Some(w) = width else {
        return s;
    };
    let char_len = s.chars().count();
    if char_len >= w {
        return s;
    }
    let pad_char = if zero_pad { '0' } else { ' ' };
    let pad: String = std::iter::repeat(pad_char).take(w - char_len).collect();
    if left_align {
        format!("{}{}", s, pad)
    } else {
        format!("{}{}", pad, s)
    }
}
