use std::{
    cell::RefCell,
    cmp::Ordering,
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::LazyLock,
};

use regex::Regex;

use crate::{
    error::ControlFlow,
    regex_cache,
    stash::is_private_enabled,
    value::{CallContext, MethodResult, Value},
};

static WHITESPACE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

/// A virtual method (`[% list.mymethod(args) %]`): called with the value it
/// was invoked on and its arguments, like a Perl `$Template::Stash::*_OPS`
/// sub.  Any `Fn(&Value, &[Value]) -> Result<Value, ControlFlow>`, closures
/// included, is a `VMethod`.
pub trait VMethod {
    fn call(&self, target: &Value, args: &[Value]) -> Result<Value, ControlFlow>;
}

impl<F: Fn(&Value, &[Value]) -> Result<Value, ControlFlow>> VMethod for F {
    fn call(&self, target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
        self(target, args)
    }
}

pub type VMethodFn = Rc<dyn VMethod>;

#[derive(Clone)]
pub struct VMethodRegistry {
    pub root_ops: HashMap<String, VMethodFn>,
    pub scalar_ops: HashMap<String, VMethodFn>,
    pub hash_ops: HashMap<String, VMethodFn>,
    pub list_ops: HashMap<String, VMethodFn>,
}

impl std::fmt::Debug for VMethodRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names = |ops: &HashMap<String, VMethodFn>| {
            let mut v: Vec<String> = ops.keys().cloned().collect();
            v.sort();
            v
        };
        f.debug_struct("VMethodRegistry")
            .field("root_ops", &names(&self.root_ops))
            .field("scalar_ops", &names(&self.scalar_ops))
            .field("hash_ops", &names(&self.hash_ops))
            .field("list_ops", &names(&self.list_ops))
            .finish()
    }
}

impl Default for VMethodRegistry {
    fn default() -> Self {
        Self::new()
    }
}

thread_local! {
    pub static GLOBAL_VMETHODS: RefCell<VMethodRegistry> = RefCell::new(VMethodRegistry::new());
}

pub fn reset_global_vmethods() {
    GLOBAL_VMETHODS.with(|g| {
        *g.borrow_mut() = VMethodRegistry::new();
    });
}

pub fn define_global_vmethod(vtype: &str, name: &str, f: impl VMethod + 'static) {
    GLOBAL_VMETHODS.with(|g| {
        g.borrow_mut().define_vmethod(vtype, name, f);
    });
}

impl VMethodRegistry {
    pub fn new() -> Self {
        let mut reg = Self {
            root_ops: HashMap::new(),
            scalar_ops: HashMap::new(),
            hash_ops: HashMap::new(),
            list_ops: HashMap::new(),
        };

        // ROOT_OPS
        reg.root_ops
            .insert("inc".to_string(), Rc::new(vmethod_root_inc));
        reg.root_ops
            .insert("dec".to_string(), Rc::new(vmethod_root_dec));
        reg.root_ops
            .insert("import".to_string(), Rc::new(vmethod_hash_import));

        // SCALAR_OPS
        reg.scalar_ops
            .insert("item".to_string(), Rc::new(vmethod_scalar_item));
        reg.scalar_ops
            .insert("list".to_string(), Rc::new(vmethod_scalar_list));
        reg.scalar_ops
            .insert("hash".to_string(), Rc::new(vmethod_scalar_hash));
        reg.scalar_ops
            .insert("length".to_string(), Rc::new(vmethod_scalar_length));
        reg.scalar_ops
            .insert("size".to_string(), Rc::new(vmethod_scalar_size));
        reg.scalar_ops
            .insert("empty".to_string(), Rc::new(vmethod_scalar_empty));
        reg.scalar_ops
            .insert("defined".to_string(), Rc::new(vmethod_scalar_defined));
        reg.scalar_ops
            .insert("upper".to_string(), Rc::new(vmethod_scalar_upper));
        reg.scalar_ops
            .insert("lower".to_string(), Rc::new(vmethod_scalar_lower));
        reg.scalar_ops
            .insert("ucfirst".to_string(), Rc::new(vmethod_scalar_ucfirst));
        reg.scalar_ops
            .insert("lcfirst".to_string(), Rc::new(vmethod_scalar_lcfirst));
        reg.scalar_ops
            .insert("trim".to_string(), Rc::new(vmethod_scalar_trim));
        reg.scalar_ops
            .insert("collapse".to_string(), Rc::new(vmethod_scalar_collapse));
        reg.scalar_ops
            .insert("squote".to_string(), Rc::new(vmethod_scalar_squote));
        reg.scalar_ops
            .insert("dquote".to_string(), Rc::new(vmethod_scalar_dquote));
        reg.scalar_ops
            .insert("html".to_string(), Rc::new(vmethod_scalar_html));
        reg.scalar_ops
            .insert("xml".to_string(), Rc::new(vmethod_scalar_xml));
        reg.scalar_ops
            .insert("repeat".to_string(), Rc::new(vmethod_scalar_repeat));
        reg.scalar_ops
            .insert("search".to_string(), Rc::new(vmethod_scalar_search));
        reg.scalar_ops
            .insert("match".to_string(), Rc::new(vmethod_scalar_match));
        reg.scalar_ops
            .insert("replace".to_string(), Rc::new(vmethod_scalar_replace));
        reg.scalar_ops
            .insert("remove".to_string(), Rc::new(vmethod_scalar_remove));
        reg.scalar_ops
            .insert("split".to_string(), Rc::new(vmethod_scalar_split));
        reg.scalar_ops
            .insert("chunk".to_string(), Rc::new(vmethod_scalar_chunk));
        reg.scalar_ops
            .insert("substr".to_string(), Rc::new(vmethod_scalar_substr));

        // HASH_OPS
        reg.hash_ops
            .insert("item".to_string(), Rc::new(vmethod_hash_item));
        reg.hash_ops
            .insert("hash".to_string(), Rc::new(vmethod_hash_hash));
        reg.hash_ops
            .insert("size".to_string(), Rc::new(vmethod_hash_size));
        reg.hash_ops
            .insert("empty".to_string(), Rc::new(vmethod_hash_empty));
        reg.hash_ops
            .insert("keys".to_string(), Rc::new(vmethod_hash_keys));
        reg.hash_ops
            .insert("values".to_string(), Rc::new(vmethod_hash_values));
        reg.hash_ops
            .insert("items".to_string(), Rc::new(vmethod_hash_items));
        reg.hash_ops
            .insert("each".to_string(), Rc::new(vmethod_hash_items));
        reg.hash_ops
            .insert("pairs".to_string(), Rc::new(vmethod_hash_pairs));
        reg.hash_ops
            .insert("list".to_string(), Rc::new(vmethod_hash_list));
        reg.hash_ops
            .insert("exists".to_string(), Rc::new(vmethod_hash_exists));
        reg.hash_ops
            .insert("defined".to_string(), Rc::new(vmethod_hash_defined));
        reg.hash_ops
            .insert("delete".to_string(), Rc::new(vmethod_hash_delete));
        reg.hash_ops
            .insert("import".to_string(), Rc::new(vmethod_hash_import));
        reg.hash_ops
            .insert("sort".to_string(), Rc::new(vmethod_hash_sort));
        reg.hash_ops
            .insert("nsort".to_string(), Rc::new(vmethod_hash_nsort));

        // LIST_OPS
        reg.list_ops
            .insert("item".to_string(), Rc::new(vmethod_list_item));
        reg.list_ops
            .insert("list".to_string(), Rc::new(vmethod_list_list));
        reg.list_ops
            .insert("hash".to_string(), Rc::new(vmethod_list_hash));
        reg.list_ops
            .insert("size".to_string(), Rc::new(vmethod_list_size));
        reg.list_ops
            .insert("empty".to_string(), Rc::new(vmethod_list_empty));
        reg.list_ops
            .insert("max".to_string(), Rc::new(vmethod_list_max));
        reg.list_ops
            .insert("defined".to_string(), Rc::new(vmethod_list_defined));
        reg.list_ops
            .insert("first".to_string(), Rc::new(vmethod_list_first));
        reg.list_ops
            .insert("last".to_string(), Rc::new(vmethod_list_last));
        reg.list_ops
            .insert("reverse".to_string(), Rc::new(vmethod_list_reverse));
        reg.list_ops
            .insert("join".to_string(), Rc::new(vmethod_list_join));
        reg.list_ops
            .insert("grep".to_string(), Rc::new(vmethod_list_grep));
        reg.list_ops
            .insert("sort".to_string(), Rc::new(vmethod_list_sort));
        reg.list_ops
            .insert("nsort".to_string(), Rc::new(vmethod_list_nsort));
        reg.list_ops
            .insert("unique".to_string(), Rc::new(vmethod_list_unique));
        reg.list_ops
            .insert("import".to_string(), Rc::new(vmethod_list_import));
        reg.list_ops
            .insert("merge".to_string(), Rc::new(vmethod_list_merge));
        reg.list_ops
            .insert("slice".to_string(), Rc::new(vmethod_list_slice));
        reg.list_ops
            .insert("splice".to_string(), Rc::new(vmethod_list_splice));
        reg.list_ops
            .insert("push".to_string(), Rc::new(vmethod_list_push));
        reg.list_ops
            .insert("pop".to_string(), Rc::new(vmethod_list_pop));
        reg.list_ops
            .insert("unshift".to_string(), Rc::new(vmethod_list_unshift));
        reg.list_ops
            .insert("shift".to_string(), Rc::new(vmethod_list_shift));

        reg
    }

    pub fn define_vmethod(&mut self, vtype: &str, name: &str, f: impl VMethod + 'static) {
        let f: VMethodFn = Rc::new(f);
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

    pub fn define(&mut self, vtype: &str, name: &str, f: impl VMethod + 'static) {
        self.define_vmethod(vtype, name, f);
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
    if let Some(re) = regex_cache::regex(pattern) {
        re.is_match(text)
    } else if let Some(re) = regex_cache::fancy(pattern) {
        re.is_match(text).unwrap_or(false)
    } else {
        text.contains(pattern)
    }
}

pub fn text_match(text: &str, pattern: &str, global: bool) -> Value {
    if let Some(re) = regex_cache::regex(pattern) {
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
    } else if let Some(re) = regex_cache::fancy(pattern) {
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

    if let Some(re) = regex_cache::regex(pattern) {
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
    } else if let Some(re) = regex_cache::fancy(pattern) {
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
        let trimmed = text.trim_start();
        if trimmed.is_empty() {
            return Vec::new();
        }
        let re = &*WHITESPACE_RE;
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
        let pat = split_pat.unwrap_or_default();
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
        if let Some(re) = regex_cache::regex(pat) {
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

// ============================================================================
// ROOT_OPS
// ============================================================================

pub fn vmethod_root_inc(_root: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    let v = args.first().map(|x| x.to_int()).unwrap_or(0);
    Ok(Value::Int(v + 1))
}

pub fn vmethod_root_dec(_root: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    let v = args.first().map(|x| x.to_int()).unwrap_or(0);
    Ok(Value::Int(v - 1))
}

// ============================================================================
// SCALAR_OPS
// ============================================================================

pub fn vmethod_scalar_item(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    Ok(target.clone())
}

pub fn vmethod_scalar_list(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    if !target.is_defined() {
        return Ok(Value::Undefined);
    }
    Ok(Value::list(vec![target.clone()]))
}

pub fn vmethod_scalar_hash(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    let mut m = HashMap::new();
    m.insert("value".to_string(), target.clone());
    Ok(Value::hash(m))
}

pub fn vmethod_scalar_length(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    Ok(Value::Int(target.to_tt_string().chars().count() as i64))
}

pub fn vmethod_scalar_size(_target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    Ok(Value::Int(1))
}

pub fn vmethod_scalar_empty(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    Ok(Value::Int(if target.to_tt_string().is_empty() {
        1
    } else {
        0
    }))
}

pub fn vmethod_scalar_defined(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    Ok(Value::Int(if target.is_defined() { 1 } else { 0 }))
}

pub fn vmethod_scalar_upper(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    Ok(Value::Str(target.to_tt_string().to_uppercase()))
}

pub fn vmethod_scalar_lower(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    Ok(Value::Str(target.to_tt_string().to_lowercase()))
}

pub fn vmethod_scalar_ucfirst(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    Ok(Value::Str(text_ucfirst(&target.to_tt_string())))
}

pub fn vmethod_scalar_lcfirst(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    Ok(Value::Str(text_lcfirst(&target.to_tt_string())))
}

pub fn vmethod_scalar_trim(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    Ok(Value::Str(text_trim(&target.to_tt_string())))
}

pub fn vmethod_scalar_collapse(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    Ok(Value::Str(text_collapse(&target.to_tt_string())))
}

pub fn vmethod_scalar_squote(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    Ok(Value::Str(text_squote(&target.to_tt_string())))
}

pub fn vmethod_scalar_dquote(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    Ok(Value::Str(text_dquote(&target.to_tt_string())))
}

pub fn vmethod_scalar_html(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    Ok(Value::Str(html_escape(&target.to_tt_string())))
}

pub fn vmethod_scalar_xml(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    Ok(Value::Str(xml_escape(&target.to_tt_string())))
}

pub fn vmethod_scalar_repeat(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    let s = target.to_tt_string();
    let count = args.first().map(|v| v.to_int()).unwrap_or(1);
    if count <= 0 {
        Ok(Value::Str(String::new()))
    } else {
        Ok(Value::Str(s.repeat(count as usize)))
    }
}

pub fn vmethod_scalar_search(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    if let Some(pat) = args.first() {
        if !pat.is_defined() {
            return Ok(target.clone());
        }
        let matched = text_search(&target.to_tt_string(), &pat.to_tt_string());
        Ok(Value::Int(if matched { 1 } else { 0 }))
    } else {
        Ok(target.clone())
    }
}

pub fn vmethod_scalar_match(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    if let Some(pat) = args.first() {
        if !pat.is_defined() {
            return Ok(target.clone());
        }
        let global = args.get(1).map(|v| v.is_truthy()).unwrap_or(false);
        Ok(text_match(
            &target.to_tt_string(),
            &pat.to_tt_string(),
            global,
        ))
    } else {
        Ok(target.clone())
    }
}

pub fn vmethod_scalar_replace(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    let s = target.to_tt_string();
    let pat = args.first().map(|v| v.to_tt_string()).unwrap_or_default();
    let rep = args.get(1).map(|v| v.to_tt_string()).unwrap_or_default();
    let global = args.get(2).map(|v| v.is_truthy()).unwrap_or(true);
    Ok(Value::Str(text_replace(&s, &pat, &rep, global)))
}

pub fn vmethod_scalar_remove(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    if let Some(pat) = args.first() {
        if !pat.is_defined() {
            return Ok(target.clone());
        }
        Ok(Value::Str(text_replace(
            &target.to_tt_string(),
            &pat.to_tt_string(),
            "",
            true,
        )))
    } else {
        Ok(target.clone())
    }
}

pub fn vmethod_scalar_split(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    let s = target.to_tt_string();
    let pat_str = args.first().and_then(|v| {
        if v.is_defined() {
            Some(v.to_tt_string())
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
    Ok(Value::list(text_split(&s, pat_str.as_deref(), limit)))
}

pub fn vmethod_scalar_chunk(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    let sz = args.first().map(|v| v.to_int()).unwrap_or(1);
    Ok(Value::list(text_chunk(&target.to_tt_string(), sz)))
}

pub fn vmethod_scalar_substr(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    let s = target.to_tt_string();
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
            Some(v.to_tt_string())
        } else {
            None
        }
    });
    Ok(Value::Str(text_substr(
        &s,
        offset,
        length,
        replacement.as_deref(),
    )))
}

// ============================================================================
// HASH_OPS
// ============================================================================

fn extract_hash_clone(target: &Value) -> HashMap<String, Value> {
    match target {
        Value::Hash(h) => h.borrow().clone(),
        Value::Object(o) => o.as_hash().map(|h| h.borrow().clone()).unwrap_or_default(),
        _ => HashMap::new(),
    }
}

pub fn vmethod_hash_item(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    let key = args.first().map(|v| v.to_tt_string()).unwrap_or_default();
    if is_private_enabled() && (key.starts_with('_') || key.starts_with('.')) {
        return Ok(Value::Undefined);
    }
    let map = extract_hash_clone(target);
    Ok(map.get(&key).cloned().unwrap_or(Value::Undefined))
}

pub fn vmethod_hash_hash(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    Ok(target.clone())
}

pub fn vmethod_hash_size(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    let map = extract_hash_clone(target);
    Ok(Value::Int(map.len() as i64))
}

pub fn vmethod_hash_empty(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    let map = extract_hash_clone(target);
    Ok(Value::Int(if map.is_empty() { 1 } else { 0 }))
}

pub fn vmethod_hash_keys(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    let map = extract_hash_clone(target);
    let keys: Vec<Value> = map.keys().cloned().map(Value::Str).collect();
    Ok(Value::list(keys))
}

pub fn vmethod_hash_values(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    let map = extract_hash_clone(target);
    let vals: Vec<Value> = map.values().cloned().collect();
    Ok(Value::list(vals))
}

pub fn vmethod_hash_items(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    let map = extract_hash_clone(target);
    let mut items = Vec::with_capacity(map.len() * 2);
    for (k, v) in map {
        items.push(Value::Str(k));
        items.push(v);
    }
    Ok(Value::list(items))
}

pub fn vmethod_hash_pairs(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    let map = extract_hash_clone(target);
    let mut keys: Vec<String> = map.keys().cloned().collect();
    keys.sort();
    let pairs: Vec<Value> = keys
        .into_iter()
        .map(|k| {
            let v = map.get(&k).cloned().unwrap_or(Value::Undefined);
            let mut m = HashMap::new();
            m.insert("key".to_string(), Value::Str(k));
            m.insert("value".to_string(), v);
            Value::hash(m)
        })
        .collect();
    Ok(Value::list(pairs))
}

pub fn vmethod_hash_list(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    let what = args.first().map(|v| v.to_tt_string()).unwrap_or_default();
    match what.as_str() {
        "keys" => vmethod_hash_keys(target, args),
        "values" => vmethod_hash_values(target, args),
        "each" => vmethod_hash_items(target, args),
        _ => vmethod_hash_pairs(target, args),
    }
}

pub fn vmethod_hash_exists(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    let map = extract_hash_clone(target);
    let key = args.first().map(|v| v.to_tt_string()).unwrap_or_default();
    Ok(Value::Int(if map.contains_key(&key) { 1 } else { 0 }))
}

pub fn vmethod_hash_defined(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    let map = extract_hash_clone(target);
    if let Some(arg) = args.first() {
        let key = arg.to_tt_string();
        let is_def = map.get(&key).map(|v| v.is_defined()).unwrap_or(false);
        Ok(Value::Int(if is_def { 1 } else { 0 }))
    } else {
        Ok(Value::Int(1))
    }
}

pub fn vmethod_hash_delete(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    if let Value::Hash(h) = target {
        let mut borrow = h.borrow_mut();
        for arg in args {
            borrow.remove(&arg.to_tt_string());
        }
    } else if let Value::Object(o) = target {
        if let Some(h) = o.as_hash() {
            let mut borrow = h.borrow_mut();
            for arg in args {
                borrow.remove(&arg.to_tt_string());
            }
        }
    }
    Ok(Value::Str(String::new()))
}

pub fn vmethod_hash_import(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    for arg in args {
        if let Value::Hash(imp) = arg {
            let imp_map = imp.borrow().clone();
            if let Value::Hash(h) = target {
                h.borrow_mut().extend(imp_map);
            } else if let Value::Object(o) = target {
                if let Some(h) = o.as_hash() {
                    h.borrow_mut().extend(imp_map);
                }
            }
        }
    }
    Ok(Value::Str(String::new()))
}

pub fn vmethod_hash_sort(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    let map = extract_hash_clone(target);
    let mut keys: Vec<String> = map.keys().cloned().collect();
    keys.sort_by(|a, b| {
        let va = map
            .get(a)
            .map(|v| v.to_tt_string().to_lowercase())
            .unwrap_or_default();
        let vb = map
            .get(b)
            .map(|v| v.to_tt_string().to_lowercase())
            .unwrap_or_default();
        va.cmp(&vb).then_with(|| a.cmp(b))
    });
    Ok(Value::list(keys.into_iter().map(Value::Str).collect()))
}

pub fn vmethod_hash_nsort(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    let map = extract_hash_clone(target);
    let mut keys: Vec<String> = map.keys().cloned().collect();
    keys.sort_by(|a, b| {
        let va = map.get(a).map(|v| v.to_number()).unwrap_or(0.0);
        let vb = map.get(b).map(|v| v.to_number()).unwrap_or(0.0);
        va.partial_cmp(&vb)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.cmp(b))
    });
    Ok(Value::list(keys.into_iter().map(Value::Str).collect()))
}

// ============================================================================
// LIST_OPS
// ============================================================================

fn extract_list_clone(target: &Value) -> Vec<Value> {
    match target {
        Value::List(l) => l.borrow().clone(),
        Value::Object(o) => o.as_list().map(|l| l.borrow().clone()).unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn get_sort_key(item: &Value, field: Option<&str>) -> String {
    match field {
        None => item.to_tt_string(),
        Some(f) => match item {
            Value::Hash(h) => h
                .borrow()
                .get(f)
                .map(|v| v.to_tt_string())
                .unwrap_or_default(),
            Value::Object(o) => match o.call_method(f, &[], CallContext::Scalar) {
                MethodResult::Ok(v) => v.to_tt_string(),
                _ => o
                    .as_hash()
                    .and_then(|h| h.borrow().get(f).cloned())
                    .map(|v| v.to_tt_string())
                    .unwrap_or_default(),
            },
            _ => item.to_tt_string(),
        },
    }
}

pub fn vmethod_list_item(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    let list_clone = extract_list_clone(target);
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
    Ok(resolved.unwrap_or(Value::Undefined))
}

pub fn vmethod_list_list(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    Ok(Value::list(extract_list_clone(target)))
}

pub fn vmethod_list_hash(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    let list_clone = extract_list_clone(target);
    if let Some(arg) = args.first() {
        if arg.is_defined() {
            let mut m = HashMap::new();
            for (count, item) in (arg.to_int()..).zip(list_clone) {
                m.insert(count.to_string(), item);
            }
            return Ok(Value::hash(m));
        }
    }
    let mut m = HashMap::new();
    let mut iter = list_clone.into_iter();
    while let Some(k) = iter.next() {
        let v = iter.next().unwrap_or(Value::Undefined);
        m.insert(k.to_tt_string(), v);
    }
    Ok(Value::hash(m))
}

pub fn vmethod_list_size(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    Ok(Value::Int(extract_list_clone(target).len() as i64))
}

pub fn vmethod_list_empty(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    Ok(Value::Int(if extract_list_clone(target).is_empty() {
        1
    } else {
        0
    }))
}

pub fn vmethod_list_max(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    Ok(Value::Int(extract_list_clone(target).len() as i64 - 1))
}

pub fn vmethod_list_defined(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    let list_clone = extract_list_clone(target);
    if let Some(arg) = args.first() {
        let s = arg.to_tt_string();
        if let Ok(idx) = s.trim().parse::<isize>() {
            let actual = if idx < 0 {
                list_clone.len() as isize + idx
            } else {
                idx
            };
            let is_def = if actual >= 0 {
                list_clone
                    .get(actual as usize)
                    .map(|v| v.is_defined())
                    .unwrap_or(false)
            } else {
                false
            };
            Ok(Value::Int(if is_def { 1 } else { 0 }))
        } else {
            Ok(Value::Int(0))
        }
    } else {
        Ok(Value::Int(1))
    }
}

pub fn vmethod_list_first(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    let list_clone = extract_list_clone(target);
    if let Some(arg) = args.first() {
        if arg.is_defined() {
            let n = (arg.to_int().max(0) as usize).min(list_clone.len());
            return Ok(Value::list(list_clone[..n].to_vec()));
        }
    }
    Ok(list_clone.first().cloned().unwrap_or(Value::Undefined))
}

pub fn vmethod_list_last(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    let list_clone = extract_list_clone(target);
    if let Some(arg) = args.first() {
        if arg.is_defined() {
            let n = (arg.to_int().max(0) as usize).min(list_clone.len());
            let start = list_clone.len() - n;
            return Ok(Value::list(list_clone[start..].to_vec()));
        }
    }
    Ok(list_clone.last().cloned().unwrap_or(Value::Undefined))
}

pub fn vmethod_list_reverse(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    let mut rev = extract_list_clone(target);
    rev.reverse();
    Ok(Value::list(rev))
}

pub fn vmethod_list_join(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    let list_clone = extract_list_clone(target);
    let joint = args
        .first()
        .and_then(|v| {
            if v.is_defined() {
                Some(v.to_tt_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| " ".to_string());
    let parts: Vec<String> = list_clone.iter().map(|v| v.to_tt_string()).collect();
    Ok(Value::Str(parts.join(&joint)))
}

pub fn vmethod_list_grep(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    let list_clone = extract_list_clone(target);
    let pat = args.first().map(|v| v.to_tt_string()).unwrap_or_default();
    let mut out = Vec::new();
    for item in list_clone {
        if text_search(&item.to_tt_string(), &pat) {
            out.push(item);
        }
    }
    Ok(Value::list(out))
}

pub fn vmethod_list_sort(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    let list_clone = extract_list_clone(target);
    if list_clone.len() <= 1 {
        return Ok(Value::list(list_clone));
    }
    let fields: Vec<String> = args
        .iter()
        .filter(|v| v.is_defined())
        .map(|v| v.to_tt_string())
        .collect();
    let mut keyed: Vec<(Vec<String>, Value)> = list_clone
        .into_iter()
        .map(|item| {
            let keys = if fields.is_empty() {
                vec![get_sort_key(&item, None).to_lowercase()]
            } else {
                fields
                    .iter()
                    .map(|f| get_sort_key(&item, Some(f)).to_lowercase())
                    .collect()
            };
            (keys, item)
        })
        .collect();
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(Value::list(keyed.into_iter().map(|(_, v)| v).collect()))
}

pub fn vmethod_list_nsort(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    let list_clone = extract_list_clone(target);
    if list_clone.len() <= 1 {
        return Ok(Value::list(list_clone));
    }
    let fields: Vec<String> = args
        .iter()
        .filter(|v| v.is_defined())
        .map(|v| v.to_tt_string())
        .collect();
    let mut keyed: Vec<(Vec<f64>, Value)> = list_clone
        .into_iter()
        .map(|item| {
            let keys = if fields.is_empty() {
                vec![Value::parse_perl_number(&get_sort_key(&item, None)).0]
            } else {
                fields
                    .iter()
                    .map(|f| Value::parse_perl_number(&get_sort_key(&item, Some(f))).0)
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
    Ok(Value::list(keyed.into_iter().map(|(_, v)| v).collect()))
}

pub fn vmethod_list_unique(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    let list_clone = extract_list_clone(target);
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for item in list_clone {
        let s = item.to_tt_string();
        if seen.insert(s) {
            out.push(item);
        }
    }
    Ok(Value::list(out))
}

pub fn vmethod_list_import(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    let mut additions = Vec::new();
    for arg in args {
        if let Value::List(l) = arg {
            additions.extend(l.borrow().iter().filter(|&v| v.is_defined()).cloned());
        } else if let Value::Object(o) = arg {
            if let Some(l) = o.as_list() {
                additions.extend(l.borrow().iter().filter(|&v| v.is_defined()).cloned());
            }
        }
    }
    if let Value::List(l) = target {
        l.borrow_mut().extend(additions);
    } else if let Value::Object(o) = target {
        if let Some(l) = o.as_list() {
            l.borrow_mut().extend(additions);
        }
    }
    Ok(target.clone())
}

pub fn vmethod_list_merge(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    // `[ @$list, grep defined, map ref eq 'ARRAY' ? @$_ : undef, @_ ]`:
    // only the (defined) items of plain lists are merged; anything else,
    // even a blessed list, maps to undef and is dropped.
    let mut out = extract_list_clone(target);
    for arg in args {
        if let Value::List(l) = arg {
            out.extend(l.borrow().iter().filter(|&v| v.is_defined()).cloned());
        }
    }
    Ok(Value::list(out))
}

pub fn vmethod_list_slice(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    let list_clone = extract_list_clone(target);
    let len = list_clone.len() as i64;
    if len == 0 {
        return Ok(Value::list(Vec::new()));
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
        Ok(Value::list(Vec::new()))
    } else {
        Ok(Value::list(
            list_clone[from as usize..=to as usize].to_vec(),
        ))
    }
}

pub fn vmethod_list_splice(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    let mut vec = extract_list_clone(target);
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
        if let Some(l) = o.as_list() {
            *l.borrow_mut() = vec;
        }
    }
    Ok(Value::list(removed))
}

pub fn vmethod_list_push(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    if let Value::List(l) = target {
        l.borrow_mut().extend_from_slice(args);
    } else if let Value::Object(o) = target {
        if let Some(l) = o.as_list() {
            l.borrow_mut().extend_from_slice(args);
        }
    }
    Ok(Value::Str(String::new()))
}

pub fn vmethod_list_pop(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    let popped = if let Value::List(l) = target {
        l.borrow_mut().pop()
    } else if let Value::Object(o) = target {
        o.as_list().and_then(|l| l.borrow_mut().pop())
    } else {
        None
    };
    Ok(popped.unwrap_or(Value::Undefined))
}

pub fn vmethod_list_unshift(target: &Value, args: &[Value]) -> Result<Value, ControlFlow> {
    if let Value::List(l) = target {
        let mut borrow = l.borrow_mut();
        for (i, item) in args.iter().cloned().enumerate() {
            borrow.insert(i, item);
        }
    } else if let Value::Object(o) = target {
        if let Some(l) = o.as_list() {
            let mut borrow = l.borrow_mut();
            for (i, item) in args.iter().cloned().enumerate() {
                borrow.insert(i, item);
            }
        }
    }
    Ok(Value::Str(String::new()))
}

pub fn vmethod_list_shift(target: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    let shifted = if let Value::List(l) = target {
        let mut borrow = l.borrow_mut();
        if borrow.is_empty() {
            None
        } else {
            Some(borrow.remove(0))
        }
    } else if let Value::Object(o) = target {
        o.as_list().and_then(|l| {
            let mut borrow = l.borrow_mut();
            if borrow.is_empty() {
                None
            } else {
                Some(borrow.remove(0))
            }
        })
    } else {
        None
    };
    Ok(shifted.unwrap_or(Value::Undefined))
}
