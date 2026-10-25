use std::{
    cell::RefCell,
    cmp::Ordering,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use fancy_regex::Regex as FancyRegex;
use regex::Regex;

use crate::{
    context::Context,
    error::ControlFlow,
    value::{CallContext, Callable, MethodResult, Value},
};

pub type VMethodFn =
    Rc<dyn Fn(&Value, &[Value], Option<&mut Context>) -> Result<Value, ControlFlow>>;

#[derive(Default, Clone)]
pub struct VMethodRegistry {
    pub root_ops: HashMap<String, VMethodFn>,
    pub scalar_ops: HashMap<String, VMethodFn>,
    pub hash_ops: HashMap<String, VMethodFn>,
    pub list_ops: HashMap<String, VMethodFn>,
}

thread_local! {
    pub static GLOBAL_VMETHODS: RefCell<VMethodRegistry> = RefCell::new(VMethodRegistry::default());
}

pub fn reset_global_vmethods() {
    GLOBAL_VMETHODS.with(|g| {
        *g.borrow_mut() = VMethodRegistry::default();
    });
}

pub fn define_global_vmethod(vtype: &str, name: &str, f: VMethodFn) {
    GLOBAL_VMETHODS.with(|g| {
        g.borrow_mut().define(vtype, name, f);
    });
}

impl VMethodRegistry {
    pub fn define(&mut self, vtype: &str, name: &str, f: VMethodFn) {
        let lower = vtype.to_ascii_lowercase();
        if lower.starts_with("scalar") || lower.starts_with("item") || lower.starts_with("text") {
            self.scalar_ops.insert(name.to_string(), f);
        } else if lower.starts_with("hash") {
            self.hash_ops.insert(name.to_string(), f);
        } else if lower.starts_with("list") || lower.starts_with("array") {
            self.list_ops.insert(name.to_string(), f);
        } else if lower.starts_with("root") {
            self.root_ops.insert(name.to_string(), f);
        }
    }
}

pub fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

pub fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

pub fn text_trim(s: &str) -> String {
    s.trim().to_string()
}

pub fn text_collapse(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(trimmed.len());
    let mut in_ws = false;
    for ch in trimmed.chars() {
        if ch.is_whitespace() {
            if !in_ws {
                out.push(' ');
                in_ws = true;
            }
        } else {
            in_ws = false;
            out.push(ch);
        }
    }
    out
}

pub fn text_ucfirst(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

pub fn text_lcfirst(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
    }
}

pub fn text_squote(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch == '\'' || ch == '\\' {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

pub fn text_dquote(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            '\n' => out.push_str("\\n"),
            _ => out.push(ch),
        }
    }
    out
}

pub fn text_search(text: &str, pattern: &str) -> bool {
    if let Ok(re) = Regex::new(pattern) {
        re.is_match(text)
    } else if let Ok(re) = FancyRegex::new(pattern) {
        re.is_match(text).unwrap_or(false)
    } else {
        text.contains(pattern)
    }
}

pub fn text_match(text: &str, pattern: &str, global: bool) -> Value {
    if let Ok(re) = Regex::new(pattern) {
        let num_groups = re.captures_len();
        let mut results = Vec::new();
        if global {
            if num_groups > 1 {
                for caps in re.captures_iter(text) {
                    for i in 1..num_groups {
                        if let Some(m) = caps.get(i) {
                            results.push(Value::Str(m.as_str().to_string()));
                        } else {
                            results.push(Value::Undefined);
                        }
                    }
                }
            } else {
                for m in re.find_iter(text) {
                    results.push(Value::Str(m.as_str().to_string()));
                }
            }
        } else if let Some(caps) = re.captures(text) {
            if num_groups > 1 {
                for i in 1..num_groups {
                    if let Some(m) = caps.get(i) {
                        results.push(Value::Str(m.as_str().to_string()));
                    } else {
                        results.push(Value::Undefined);
                    }
                }
            } else {
                results.push(Value::Int(1));
            }
        }
        if results.is_empty() {
            Value::Str(String::new())
        } else {
            Value::list(results)
        }
    } else if let Ok(re) = FancyRegex::new(pattern) {
        let num_groups = re.captures_len();
        let mut results = Vec::new();
        if global {
            for caps in re.captures_iter(text).flatten() {
                if num_groups > 1 {
                    for i in 1..num_groups {
                        if let Some(m) = caps.get(i) {
                            results.push(Value::Str(m.as_str().to_string()));
                        } else {
                            results.push(Value::Undefined);
                        }
                    }
                } else if let Some(m) = caps.get(0) {
                    results.push(Value::Str(m.as_str().to_string()));
                }
            }
        } else if let Ok(Some(caps)) = re.captures(text) {
            if num_groups > 1 {
                for i in 1..num_groups {
                    if let Some(m) = caps.get(i) {
                        results.push(Value::Str(m.as_str().to_string()));
                    } else {
                        results.push(Value::Undefined);
                    }
                }
            } else {
                results.push(Value::Int(1));
            }
        }
        if results.is_empty() {
            Value::Str(String::new())
        } else {
            Value::list(results)
        }
    } else {
        Value::Str(String::new())
    }
}

fn expand_replacement(replace: &str, groups: &[Option<&str>]) -> String {
    let mut out = String::with_capacity(replace.len());
    let bytes = replace.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\'
            && i + 1 < bytes.len()
            && (bytes[i + 1] == b'\\' || bytes[i + 1] == b'$')
        {
            out.push(bytes[i + 1] as char);
            i += 2;
        } else if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit() {
            let start_digit = i + 1;
            let mut end_digit = start_digit;
            while end_digit < bytes.len() && bytes[end_digit].is_ascii_digit() {
                end_digit += 1;
            }
            if let Ok(idx) = replace[start_digit..end_digit].parse::<usize>() {
                if idx > 0 {
                    if let Some(Some(val)) = groups.get(idx) {
                        out.push_str(val);
                    }
                }
            }
            i = end_digit;
        } else {
            let ch = replace[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

pub fn text_replace(text: &str, pattern: &str, replace: &str, global: bool) -> String {
    if pattern.is_empty() {
        if global {
            let mut out = String::new();
            out.push_str(replace);
            for ch in text.chars() {
                out.push(ch);
                out.push_str(replace);
            }
            return out;
        } else {
            return format!("{}{}", replace, text);
        }
    }

    let has_backrefs = {
        let bytes = replace.as_bytes();
        let mut found = false;
        for w in bytes.windows(2) {
            if w[0] == b'$' && w[1].is_ascii_digit() {
                found = true;
                break;
            }
        }
        found
    };

    if let Ok(re) = Regex::new(pattern) {
        if !has_backrefs {
            if global {
                let mut out = String::with_capacity(text.len());
                let mut last_end = 0;
                for m in re.find_iter(text) {
                    out.push_str(&text[last_end..m.start()]);
                    out.push_str(replace);
                    last_end = m.end();
                }
                out.push_str(&text[last_end..]);
                return out;
            } else if let Some(m) = re.find(text) {
                let mut out = String::with_capacity(text.len());
                out.push_str(&text[..m.start()]);
                out.push_str(replace);
                out.push_str(&text[m.end()..]);
                return out;
            } else {
                return text.to_string();
            }
        } else {
            let mut out = String::with_capacity(text.len());
            let mut last_end = 0;
            for caps in re.captures_iter(text) {
                let m0 = caps.get(0).unwrap();
                out.push_str(&text[last_end..m0.start()]);
                let groups: Vec<Option<&str>> = (0..caps.len())
                    .map(|idx| caps.get(idx).map(|m| m.as_str()))
                    .collect();
                out.push_str(&expand_replacement(replace, &groups));
                last_end = m0.end();
                if !global {
                    break;
                }
            }
            out.push_str(&text[last_end..]);
            return out;
        }
    } else if let Ok(re) = FancyRegex::new(pattern) {
        let mut out = String::with_capacity(text.len());
        let mut last_end = 0;
        for caps in re.captures_iter(text).flatten() {
            let m0 = caps.get(0).unwrap();
            out.push_str(&text[last_end..m0.start()]);
            if has_backrefs {
                let groups: Vec<Option<&str>> = (0..caps.len())
                    .map(|idx| caps.get(idx).map(|m| m.as_str()))
                    .collect();
                out.push_str(&expand_replacement(replace, &groups));
            } else {
                out.push_str(replace);
            }
            last_end = m0.end();
            if !global {
                break;
            }
        }
        out.push_str(&text[last_end..]);
        return out;
    }

    text.to_string()
}

pub fn text_split(text: &str, split_pat: Option<&str>, limit: Option<i64>) -> Vec<Value> {
    let lim = limit.unwrap_or(0);
    if split_pat.is_none() {
        // Perl's split(' ', $str, $limit): skips leading whitespace, splits on \s+
        let trimmed = text.trim_start();
        if trimmed.is_empty() {
            return Vec::new();
        }
        let re = Regex::new(r"\s+").unwrap();
        if lim > 0 {
            re.splitn(trimmed, lim as usize)
                .map(|s| Value::Str(s.to_string()))
                .collect()
        } else {
            let mut parts: Vec<&str> = re.split(trimmed).collect();
            if lim == 0 {
                while parts.last() == Some(&"") {
                    parts.pop();
                }
            }
            parts
                .into_iter()
                .map(|s| Value::Str(s.to_string()))
                .collect()
        }
    } else {
        let pat = split_pat.unwrap();
        if pat.is_empty() {
            if lim > 0 {
                let mut chars = text.chars();
                let mut out = Vec::new();
                for _ in 0..(lim as usize).saturating_sub(1) {
                    if let Some(c) = chars.next() {
                        out.push(Value::Str(c.to_string()));
                    } else {
                        break;
                    }
                }
                let rest = chars.as_str();
                if !rest.is_empty() || out.is_empty() {
                    out.push(Value::Str(rest.to_string()));
                }
                return out;
            } else {
                return text.chars().map(|c| Value::Str(c.to_string())).collect();
            }
        }
        if let Ok(re) = Regex::new(pat) {
            if lim > 0 {
                re.splitn(text, lim as usize)
                    .map(|s| Value::Str(s.to_string()))
                    .collect()
            } else {
                let mut parts: Vec<&str> = re.split(text).collect();
                if lim == 0 {
                    while parts.last() == Some(&"") {
                        parts.pop();
                    }
                }
                parts
                    .into_iter()
                    .map(|s| Value::Str(s.to_string()))
                    .collect()
            }
        } else {
            text.split(pat).map(|s| Value::Str(s.to_string())).collect()
        }
    }
}

pub fn text_chunk(text: &str, size: i64) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }
    let sz = if size == 0 { 1 } else { size };
    let chars: Vec<char> = text.chars().collect();
    if sz > 0 {
        let step = sz as usize;
        chars
            .chunks(step)
            .map(|c| Value::Str(c.iter().collect()))
            .collect()
    } else {
        let step = (-sz) as usize;
        let mut rev_chunks: Vec<Value> = chars
            .rchunks(step)
            .map(|c| Value::Str(c.iter().collect()))
            .collect();
        rev_chunks.reverse();
        rev_chunks
    }
}

pub fn text_substr(
    text: &str,
    offset: i64,
    length: Option<i64>,
    replacement: Option<&str>,
) -> String {
    let chars: Vec<char> = text.chars().collect();
    let total = chars.len() as i64;
    let start = if offset < 0 {
        (total + offset).max(0) as usize
    } else {
        (offset.min(total)) as usize
    };
    let end = match length {
        None => chars.len(),
        Some(len) if len < 0 => (total + len).max(start as i64) as usize,
        Some(len) => ((start as i64 + len).min(total)).max(start as i64) as usize,
    };

    if let Some(rep) = replacement {
        let mut out: String = chars[..start].iter().collect();
        out.push_str(rep);
        out.extend(&chars[end..]);
        out
    } else {
        chars[start..end].iter().collect()
    }
}

pub fn call_scalar_vmethod(
    target: &Value,
    method: &str,
    args: &[Value],
    mut ctx: Option<&mut Context>,
) -> MethodResult {
    // Check context-defined and global scalar vmethods first
    if let Some(ref mut c) = ctx {
        if let Some(f) = c.vmethods.scalar_ops.get(method).cloned() {
            return match f(target, args, Some(*c)) {
                Ok(v) => MethodResult::Value(v),
                Err(e) => MethodResult::Error(e),
            };
        }
    }
    let global_fn = GLOBAL_VMETHODS.with(|g| g.borrow().scalar_ops.get(method).cloned());
    if let Some(f) = global_fn {
        return match f(target, args, ctx) {
            Ok(v) => MethodResult::Value(v),
            Err(e) => MethodResult::Error(e),
        };
    }

    // Special check forUndefined target
    if matches!(target, Value::Undefined) {
        return match method {
            "defined" => MethodResult::Value(Value::Int(0)),
            _ => MethodResult::NotFound,
        };
    }

    let s = target.to_str();
    match method {
        "item" => MethodResult::Value(target.clone()),
        "list" => MethodResult::Value(Value::list(vec![target.clone()])),
        "hash" => {
            let mut m = HashMap::new();
            m.insert("value".to_string(), target.clone());
            MethodResult::Value(Value::hash(m))
        }
        "length" => MethodResult::Value(Value::Int(s.chars().count() as i64)),
        "size" => MethodResult::Value(Value::Int(1)),
        "empty" => MethodResult::Value(Value::Int(if s.is_empty() { 1 } else { 0 })),
        "defined" => MethodResult::Value(Value::Int(if target.is_defined() { 1 } else { 0 })),
        "upper" => MethodResult::Value(Value::Str(s.to_uppercase())),
        "lower" => MethodResult::Value(Value::Str(s.to_lowercase())),
        "ucfirst" => MethodResult::Value(Value::Str(text_ucfirst(&s))),
        "lcfirst" => MethodResult::Value(Value::Str(text_lcfirst(&s))),
        "trim" => MethodResult::Value(Value::Str(text_trim(&s))),
        "collapse" => MethodResult::Value(Value::Str(text_collapse(&s))),
        "squote" => MethodResult::Value(Value::Str(text_squote(&s))),
        "dquote" => MethodResult::Value(Value::Str(text_dquote(&s))),
        "html" => MethodResult::Value(Value::Str(html_escape(&s))),
        "xml" => MethodResult::Value(Value::Str(xml_escape(&s))),
        "repeat" => {
            let count = args.first().map(|v| v.to_int()).unwrap_or(1);
            if count <= 0 {
                MethodResult::Value(Value::Str(String::new()))
            } else {
                MethodResult::Value(Value::Str(s.repeat(count as usize)))
            }
        }
        "search" => {
            if let Some(pat) = args.first() {
                if !pat.is_defined() {
                    return MethodResult::Value(target.clone());
                }
                let matched = text_search(&s, &pat.to_str());
                MethodResult::Value(Value::Int(if matched { 1 } else { 0 }))
            } else {
                MethodResult::Value(target.clone())
            }
        }
        "match" => {
            if let Some(pat) = args.first() {
                if !pat.is_defined() {
                    return MethodResult::Value(target.clone());
                }
                let global = args.get(1).map(|v| v.is_truthy()).unwrap_or(false);
                MethodResult::Value(text_match(&s, &pat.to_str(), global))
            } else {
                MethodResult::Value(target.clone())
            }
        }
        "replace" => {
            let pat = args.first().map(|v| v.to_str()).unwrap_or_default();
            let rep = args.get(1).map(|v| v.to_str()).unwrap_or_default();
            let global = args.get(2).map(|v| v.is_truthy()).unwrap_or(true);
            MethodResult::Value(Value::Str(text_replace(&s, &pat, &rep, global)))
        }
        "remove" => {
            if let Some(pat) = args.first() {
                if !pat.is_defined() {
                    return MethodResult::Value(target.clone());
                }
                MethodResult::Value(Value::Str(text_replace(&s, &pat.to_str(), "", true)))
            } else {
                MethodResult::Value(target.clone())
            }
        }
        "split" => {
            let pat_str = args.first().and_then(|v| {
                if v.is_defined() {
                    Some(v.to_str())
                } else {
                    None
                }
            });
            let limit = args.get(1).and_then(|v| {
                if v.is_defined() {
                    Some(v.to_int())
                } else {
                    None
                }
            });
            MethodResult::Value(Value::list(text_split(&s, pat_str.as_deref(), limit)))
        }
        "chunk" => {
            let sz = args.first().map(|v| v.to_int()).unwrap_or(1);
            MethodResult::Value(Value::list(text_chunk(&s, sz)))
        }
        "substr" => {
            let offset = args.first().map(|v| v.to_int()).unwrap_or(0);
            let length = args.get(1).and_then(|v| {
                if v.is_defined() {
                    Some(v.to_int())
                } else {
                    None
                }
            });
            let replacement = args.get(2).and_then(|v| {
                if v.is_defined() {
                    Some(v.to_str())
                } else {
                    None
                }
            });
            MethodResult::Value(Value::Str(text_substr(
                &s,
                offset,
                length,
                replacement.as_deref(),
            )))
        }
        _ => MethodResult::NotFound,
    }
}

pub fn call_hash_vmethod(
    target: &Value,
    method: &str,
    args: &[Value],
    mut ctx: Option<&mut Context>,
    private_regex: bool,
) -> MethodResult {
    if let Some(ref mut c) = ctx {
        if let Some(f) = c.vmethods.hash_ops.get(method).cloned() {
            return match f(target, args, Some(*c)) {
                Ok(v) => MethodResult::Value(v),
                Err(e) => MethodResult::Error(e),
            };
        }
    }
    let global_fn = GLOBAL_VMETHODS.with(|g| g.borrow().hash_ops.get(method).cloned());
    if let Some(f) = global_fn {
        return match f(target, args, ctx) {
            Ok(v) => MethodResult::Value(v),
            Err(e) => MethodResult::Error(e),
        };
    }

    // Extract hash map (either Value::Hash or Value::Object with as_hash)
    let hash_clone: HashMap<String, Value> = match target {
        Value::Hash(h) => h.borrow().clone(),
        Value::Object(o) => {
            if let Some(h) = o.borrow().as_hash() {
                h.clone()
            } else {
                return MethodResult::NotFound;
            }
        }
        _ => return MethodResult::NotFound,
    };

    match method {
        "item" => {
            let key = args.first().map(|v| v.to_str()).unwrap_or_default();
            if private_regex && (key.starts_with('_') || key.starts_with('.')) {
                return MethodResult::Value(Value::Undefined);
            }
            MethodResult::Value(hash_clone.get(&key).cloned().unwrap_or(Value::Undefined))
        }
        "hash" => MethodResult::Value(target.clone()),
        "size" => MethodResult::Value(Value::Int(hash_clone.len() as i64)),
        "empty" => MethodResult::Value(Value::Int(if hash_clone.is_empty() { 1 } else { 0 })),
        "keys" => {
            let keys: Vec<Value> = hash_clone.keys().cloned().map(Value::Str).collect();
            MethodResult::Value(Value::list(keys))
        }
        "values" => {
            let vals: Vec<Value> = hash_clone.values().cloned().collect();
            MethodResult::Value(Value::list(vals))
        }
        "items" | "each" => {
            let mut items = Vec::with_capacity(hash_clone.len() * 2);
            for (k, v) in hash_clone {
                items.push(Value::Str(k));
                items.push(v);
            }
            MethodResult::Value(Value::list(items))
        }
        "pairs" => {
            let mut keys: Vec<String> = hash_clone.keys().cloned().collect();
            keys.sort();
            let pairs: Vec<Value> = keys
                .into_iter()
                .map(|k| {
                    let v = hash_clone.get(&k).cloned().unwrap_or(Value::Undefined);
                    let mut m = HashMap::new();
                    m.insert("key".to_string(), Value::Str(k));
                    m.insert("value".to_string(), v);
                    Value::hash(m)
                })
                .collect();
            MethodResult::Value(Value::list(pairs))
        }
        "list" => {
            let what = args.first().map(|v| v.to_str()).unwrap_or_default();
            match what.as_str() {
                "keys" => {
                    let keys: Vec<Value> = hash_clone.keys().cloned().map(Value::Str).collect();
                    MethodResult::Value(Value::list(keys))
                }
                "values" => {
                    let vals: Vec<Value> = hash_clone.values().cloned().collect();
                    MethodResult::Value(Value::list(vals))
                }
                "each" => {
                    let mut items = Vec::with_capacity(hash_clone.len() * 2);
                    for (k, v) in hash_clone {
                        items.push(Value::Str(k));
                        items.push(v);
                    }
                    MethodResult::Value(Value::list(items))
                }
                _ => {
                    let mut keys: Vec<String> = hash_clone.keys().cloned().collect();
                    keys.sort();
                    let pairs: Vec<Value> = keys
                        .into_iter()
                        .map(|k| {
                            let v = hash_clone.get(&k).cloned().unwrap_or(Value::Undefined);
                            let mut m = HashMap::new();
                            m.insert("key".to_string(), Value::Str(k));
                            m.insert("value".to_string(), v);
                            Value::hash(m)
                        })
                        .collect();
                    MethodResult::Value(Value::list(pairs))
                }
            }
        }
        "exists" => {
            let key = args.first().map(|v| v.to_str()).unwrap_or_default();
            MethodResult::Value(Value::Int(if hash_clone.contains_key(&key) {
                1
            } else {
                0
            }))
        }
        "defined" => {
            if let Some(arg) = args.first() {
                let key = arg.to_str();
                let is_def = hash_clone
                    .get(&key)
                    .map(|v| v.is_defined())
                    .unwrap_or(false);
                MethodResult::Value(Value::Int(if is_def { 1 } else { 0 }))
            } else {
                MethodResult::Value(Value::Int(1))
            }
        }
        "delete" => {
            if let Value::Hash(h) = target {
                let mut borrow = h.borrow_mut();
                for arg in args {
                    borrow.remove(&arg.to_str());
                }
            } else if let Value::Object(o) = target {
                let mut borrow = o.borrow_mut();
                if let Some(h) = borrow.as_hash_mut() {
                    for arg in args {
                        h.remove(&arg.to_str());
                    }
                }
            }
            MethodResult::Value(Value::Str(String::new()))
        }
        "import" => {
            if let Some(Value::Hash(imp)) = args.first() {
                let imp_map = imp.borrow().clone();
                if let Value::Hash(h) = target {
                    h.borrow_mut().extend(imp_map);
                } else if let Value::Object(o) = target {
                    if let Some(h) = o.borrow_mut().as_hash_mut() {
                        h.extend(imp_map);
                    }
                }
            }
            MethodResult::Value(Value::Str(String::new()))
        }
        "sort" => {
            // Sort keys by values case-insensitively
            let mut keys: Vec<String> = hash_clone.keys().cloned().collect();
            keys.sort_by(|a, b| {
                let va = hash_clone
                    .get(a)
                    .map(|v| v.to_str().to_lowercase())
                    .unwrap_or_default();
                let vb = hash_clone
                    .get(b)
                    .map(|v| v.to_str().to_lowercase())
                    .unwrap_or_default();
                va.cmp(&vb).then_with(|| a.cmp(b))
            });
            MethodResult::Value(Value::list(keys.into_iter().map(Value::Str).collect()))
        }
        "nsort" => {
            // Sort keys by values numerically
            let mut keys: Vec<String> = hash_clone.keys().cloned().collect();
            keys.sort_by(|a, b| {
                let va = hash_clone.get(a).map(|v| v.to_number()).unwrap_or(0.0);
                let vb = hash_clone.get(b).map(|v| v.to_number()).unwrap_or(0.0);
                va.partial_cmp(&vb)
                    .unwrap_or(Ordering::Equal)
                    .then_with(|| a.cmp(b))
            });
            MethodResult::Value(Value::list(keys.into_iter().map(Value::Str).collect()))
        }
        _ => MethodResult::NotFound,
    }
}

fn get_sort_key(item: &Value, field: Option<&str>, ctx: &mut Option<&mut Context>) -> String {
    match field {
        None => item.to_str(),
        Some(f) => match item {
            Value::Hash(h) => h.borrow().get(f).map(|v| v.to_str()).unwrap_or_default(),
            Value::Object(o) => {
                let ctx_reborrow = ctx.as_deref_mut();
                let res = o
                    .borrow_mut()
                    .call_method(f, &[], CallContext::List, ctx_reborrow);
                match res {
                    MethodResult::Value(v) => v.to_str(),
                    _ => o
                        .borrow()
                        .as_hash()
                        .and_then(|h| h.get(f))
                        .map(|v| v.to_str())
                        .unwrap_or_default(),
                }
            }
            _ => item.to_str(),
        },
    }
}

pub fn call_list_vmethod(
    target: &Value,
    method: &str,
    args: &[Value],
    mut ctx: Option<&mut Context>,
) -> MethodResult {
    if let Some(ref mut c) = ctx {
        if let Some(f) = c.vmethods.list_ops.get(method).cloned() {
            return match f(target, args, Some(*c)) {
                Ok(v) => MethodResult::Value(v),
                Err(e) => MethodResult::Error(e),
            };
        }
    }
    let global_fn = GLOBAL_VMETHODS.with(|g| g.borrow().list_ops.get(method).cloned());
    if let Some(f) = global_fn {
        return match f(target, args, ctx) {
            Ok(v) => MethodResult::Value(v),
            Err(e) => MethodResult::Error(e),
        };
    }

    let list_clone: Vec<Value> = match target {
        Value::List(l) => l.borrow().clone(),
        Value::Object(o) => {
            if let Some(l) = o.borrow().as_list() {
                l.clone()
            } else {
                return MethodResult::NotFound;
            }
        }
        _ => return MethodResult::NotFound,
    };

    match method {
        "item" => {
            let idx = args.first().map(|v| v.to_int()).unwrap_or(0);
            let resolved = if idx < 0 {
                let pos = list_clone.len() as i64 + idx;
                if pos < 0 {
                    None
                } else {
                    list_clone.get(pos as usize).cloned()
                }
            } else {
                list_clone.get(idx as usize).cloned()
            };
            MethodResult::Value(resolved.unwrap_or(Value::Undefined))
        }
        "list" => MethodResult::Value(Value::list(list_clone)),
        "hash" => {
            if let Some(arg) = args.first() {
                if arg.is_defined() {
                    let mut count = arg.to_int();
                    let mut m = HashMap::new();
                    for item in list_clone {
                        m.insert(count.to_string(), item);
                        count += 1;
                    }
                    return MethodResult::Value(Value::hash(m));
                }
            }
            let mut m = HashMap::new();
            let mut iter = list_clone.into_iter();
            while let Some(k) = iter.next() {
                let v = iter.next().unwrap_or(Value::Undefined);
                m.insert(k.to_str(), v);
            }
            MethodResult::Value(Value::hash(m))
        }
        "size" => MethodResult::Value(Value::Int(list_clone.len() as i64)),
        "empty" => MethodResult::Value(Value::Int(if list_clone.is_empty() { 1 } else { 0 })),
        "max" => MethodResult::Value(Value::Int(list_clone.len() as i64 - 1)),
        "defined" => {
            if let Some(arg) = args.first() {
                let s = arg.to_str();
                if let Ok(idx) = s.trim().parse::<usize>() {
                    let is_def = list_clone.get(idx).map(|v| v.is_defined()).unwrap_or(false);
                    MethodResult::Value(Value::Int(if is_def { 1 } else { 0 }))
                } else {
                    MethodResult::Value(Value::Int(0))
                }
            } else {
                MethodResult::Value(Value::Int(1))
            }
        }
        "first" => {
            if let Some(arg) = args.first() {
                if arg.is_defined() {
                    let n = (arg.to_int().max(0) as usize).min(list_clone.len());
                    return MethodResult::Value(Value::list(list_clone[..n].to_vec()));
                }
            }
            MethodResult::Value(list_clone.first().cloned().unwrap_or(Value::Undefined))
        }
        "last" => {
            if let Some(arg) = args.first() {
                if arg.is_defined() {
                    let n = (arg.to_int().max(0) as usize).min(list_clone.len());
                    let start = list_clone.len() - n;
                    return MethodResult::Value(Value::list(list_clone[start..].to_vec()));
                }
            }
            MethodResult::Value(list_clone.last().cloned().unwrap_or(Value::Undefined))
        }
        "reverse" => {
            let mut rev = list_clone;
            rev.reverse();
            MethodResult::Value(Value::list(rev))
        }
        "join" => {
            let joint = args
                .first()
                .and_then(|v| {
                    if v.is_defined() {
                        Some(v.to_str())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| " ".to_string());
            let parts: Vec<String> = list_clone.iter().map(|v| v.to_str()).collect();
            MethodResult::Value(Value::Str(parts.join(&joint)))
        }
        "grep" => {
            let pat = args.first().map(|v| v.to_str()).unwrap_or_default();
            let mut out = Vec::new();
            for item in list_clone {
                if text_search(&item.to_str(), &pat) {
                    out.push(item);
                }
            }
            MethodResult::Value(Value::list(out))
        }
        "sort" => {
            if list_clone.len() <= 1 {
                return MethodResult::Value(Value::list(list_clone));
            }
            let fields: Vec<String> = args
                .iter()
                .filter(|v| v.is_defined())
                .map(|v| v.to_str())
                .collect();
            let mut keyed: Vec<(Vec<String>, Value)> = list_clone
                .into_iter()
                .map(|item| {
                    let keys = if fields.is_empty() {
                        vec![get_sort_key(&item, None, &mut ctx).to_lowercase()]
                    } else {
                        fields
                            .iter()
                            .map(|f| get_sort_key(&item, Some(f), &mut ctx).to_lowercase())
                            .collect()
                    };
                    (keys, item)
                })
                .collect();
            keyed.sort_by(|a, b| a.0.cmp(&b.0));
            MethodResult::Value(Value::list(keyed.into_iter().map(|(_, v)| v).collect()))
        }
        "nsort" => {
            if list_clone.len() <= 1 {
                return MethodResult::Value(Value::list(list_clone));
            }
            let fields: Vec<String> = args
                .iter()
                .filter(|v| v.is_defined())
                .map(|v| v.to_str())
                .collect();
            let mut keyed: Vec<(Vec<f64>, Value)> = list_clone
                .into_iter()
                .map(|item| {
                    let keys = if fields.is_empty() {
                        vec![Value::parse_perl_number(&get_sort_key(&item, None, &mut ctx)).0]
                    } else {
                        fields
                            .iter()
                            .map(|f| {
                                Value::parse_perl_number(&get_sort_key(&item, Some(f), &mut ctx)).0
                            })
                            .collect()
                    };
                    (keys, item)
                })
                .collect();
            keyed.sort_by(|a, b| {
                for (ka, kb) in a.0.iter().zip(b.0.iter()) {
                    let ord = ka.partial_cmp(kb).unwrap_or(Ordering::Equal);
                    if ord != Ordering::Equal {
                        return ord;
                    }
                }
                Ordering::Equal
            });
            MethodResult::Value(Value::list(keyed.into_iter().map(|(_, v)| v).collect()))
        }
        "unique" => {
            let mut seen = HashSet::new();
            let mut out = Vec::new();
            for item in list_clone {
                let s = item.to_str();
                if seen.insert(s) {
                    out.push(item);
                }
            }
            MethodResult::Value(Value::list(out))
        }
        "import" => {
            let mut additions = Vec::new();
            for arg in args {
                if let Value::List(l) = arg {
                    additions.extend(l.borrow().iter().cloned().filter(|v| v.is_defined()));
                }
            }
            if let Value::List(l) = target {
                l.borrow_mut().extend(additions);
            } else if let Value::Object(o) = target {
                if let Some(l) = o.borrow_mut().as_list_mut() {
                    l.extend(additions);
                }
            }
            MethodResult::Value(target.clone())
        }
        "merge" => {
            let mut out = list_clone;
            for arg in args {
                if let Value::List(l) = arg {
                    out.extend(l.borrow().iter().cloned().filter(|v| v.is_defined()));
                } else if arg.is_defined() {
                    out.push(arg.clone());
                }
            }
            MethodResult::Value(Value::list(out))
        }
        "slice" => {
            let len = list_clone.len() as i64;
            if len == 0 {
                return MethodResult::Value(Value::list(Vec::new()));
            }
            let mut from = args
                .first()
                .and_then(|v| {
                    if v.is_defined() {
                        Some(v.to_int())
                    } else {
                        None
                    }
                })
                .unwrap_or(0);
            let mut to = args
                .get(1)
                .and_then(|v| {
                    if v.is_defined() {
                        Some(v.to_int())
                    } else {
                        None
                    }
                })
                .unwrap_or(len - 1);
            if from < 0 {
                from += len;
            }
            if from < 0 {
                from = 0;
            }
            if to < 0 {
                to += len;
            }
            if to >= len {
                to = len - 1;
            }
            if from > to || from >= len {
                MethodResult::Value(Value::list(Vec::new()))
            } else {
                MethodResult::Value(Value::list(
                    list_clone[from as usize..=to as usize].to_vec(),
                ))
            }
        }
        "splice" => {
            let mut vec = list_clone;
            let len = vec.len() as i64;
            let removed = if args.is_empty() {
                std::mem::take(&mut vec)
            } else {
                let mut offset = args[0].to_int();
                if offset < 0 {
                    offset = (len + offset).max(0);
                }
                let start = (offset as usize).min(vec.len());
                if args.len() == 1 {
                    vec.split_off(start)
                } else {
                    let length = args[1].to_int();
                    let end = if length < 0 {
                        ((len + length).max(start as i64) as usize).min(vec.len())
                    } else {
                        (start + length as usize).min(vec.len())
                    };
                    let mut replace_items = Vec::new();
                    for rep in &args[2..] {
                        if let Value::List(l) = rep {
                            replace_items.extend(l.borrow().iter().cloned());
                        } else {
                            replace_items.push(rep.clone());
                        }
                    }
                    vec.splice(start..end, replace_items).collect()
                }
            };
            if let Value::List(l) = target {
                *l.borrow_mut() = vec;
            } else if let Value::Object(o) = target {
                if let Some(l) = o.borrow_mut().as_list_mut() {
                    *l = vec;
                }
            }
            MethodResult::Value(Value::list(removed))
        }
        "push" => {
            if let Value::List(l) = target {
                l.borrow_mut().extend_from_slice(args);
            } else if let Value::Object(o) = target {
                if let Some(l) = o.borrow_mut().as_list_mut() {
                    l.extend_from_slice(args);
                }
            }
            MethodResult::Value(Value::Str(String::new()))
        }
        "pop" => {
            let popped = if let Value::List(l) = target {
                l.borrow_mut().pop()
            } else if let Value::Object(o) = target {
                o.borrow_mut().as_list_mut().and_then(|l| l.pop())
            } else {
                None
            };
            MethodResult::Value(popped.unwrap_or(Value::Undefined))
        }
        "unshift" => {
            if let Value::List(l) = target {
                let mut borrow = l.borrow_mut();
                for (i, item) in args.iter().cloned().enumerate() {
                    borrow.insert(i, item);
                }
            } else if let Value::Object(o) = target {
                if let Some(l) = o.borrow_mut().as_list_mut() {
                    for (i, item) in args.iter().cloned().enumerate() {
                        l.insert(i, item);
                    }
                }
            }
            MethodResult::Value(Value::Str(String::new()))
        }
        "shift" => {
            let shifted = if let Value::List(l) = target {
                let mut borrow = l.borrow_mut();
                if borrow.is_empty() {
                    None
                } else {
                    Some(borrow.remove(0))
                }
            } else if let Value::Object(o) = target {
                o.borrow_mut().as_list_mut().and_then(|l| {
                    if l.is_empty() {
                        None
                    } else {
                        Some(l.remove(0))
                    }
                })
            } else {
                None
            };
            MethodResult::Value(shifted.unwrap_or(Value::Undefined))
        }
        _ => MethodResult::NotFound,
    }
}

pub fn make_root_ops() -> HashMap<String, Value> {
    let mut map = HashMap::new();
    map.insert(
        "inc".to_string(),
        Value::function(|args, _, _| {
            let v = args.first().map(|x| x.to_int()).unwrap_or(0);
            Ok(Value::Int(v + 1))
        }),
    );
    map.insert(
        "dec".to_string(),
        Value::function(|args, _, _| {
            let v = args.first().map(|x| x.to_int()).unwrap_or(0);
            Ok(Value::Int(v - 1))
        }),
    );
    map.insert(
        "import".to_string(),
        Value::function(|args, _, ctx| {
            if let (Some(Value::Hash(h)), Some(c)) = (args.first(), ctx) {
                for (k, v) in h.borrow().iter() {
                    c.stash.root_insert(k.clone(), v.clone());
                }
            }
            Ok(Value::Str(String::new()))
        }),
    );
    map
}
