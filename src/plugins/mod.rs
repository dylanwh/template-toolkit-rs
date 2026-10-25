use std::{
    cell::RefCell,
    collections::HashMap,
    fs,
    os::unix::fs::MetadataExt,
    path::Path,
    rc::Rc,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    context::Context,
    error::{ControlFlow, TemplateException},
    filters::{html_filter, perl_sprintf, uri_filter, url_filter, FilterDef, FilterFn},
    iterator::TemplateIterator,
    strftime,
    value::{format_perl_float, CallContext, MethodResult, TemplateObject, Value},
    view::View,
    vmethods::{vmethod_scalar_split, vmethod_scalar_substr, GLOBAL_VMETHODS},
};

pub static URL_JOINT: Mutex<&'static str> = Mutex::new("&amp;");

pub fn set_url_joint(joint: &'static str) {
    if let Ok(mut g) = URL_JOINT.lock() {
        *g = joint;
    }
}

pub fn get_url_joint() -> String {
    URL_JOINT
        .lock()
        .map(|g| (*g).to_string())
        .unwrap_or_else(|_| "&amp;".to_string())
}

pub fn load_builtin_plugin(
    name: &str,
    args: &[Value],
    context: &Context,
    plugin_base: &[String],
) -> Result<Option<Value>, TemplateException> {
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "assert" => Ok(Some(load_assert_plugin(context))),
        "scalar" => Ok(Some(load_scalar_plugin(context))),
        "cgi" => Ok(Some(load_cgi_plugin(args))),
        "datafile" => load_datafile_plugin(args).map(Some),
        "date" => Ok(Some(load_date_plugin(args))),
        "directory" => load_directory_plugin(args, context).map(Some),
        "file" => load_file_plugin(args).map(Some),
        "format" => Ok(Some(load_format_plugin(args))),
        "html" => Ok(Some(load_html_plugin(args))),
        "image" => load_image_plugin(args).map(Some),
        "iterator" => Ok(Some(load_iterator_plugin(args))),
        "math" => Ok(Some(load_math_plugin())),
        "string" => load_string_plugin(args, context).map(Some),
        "table" => load_table_plugin(args).map(Some),
        "url" => Ok(Some(load_url_plugin(args))),
        "view" => load_view_plugin(args, context).map(Some),
        "wrap" => Ok(Some(load_wrap_plugin(context))),
        _ => {
            // Check custom test plugins if registered or declined
            let _ = plugin_base;
            Ok(None)
        }
    }
}

// ============================================================================
// 1. Assert Plugin
// ============================================================================

fn assert_vmethod(val: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    Ok(Value::Object(Rc::new(AssertMonad {
        target: Some(val.clone()),
        context: None,
    })))
}

struct AssertMonad {
    target: Option<Value>,
    context: Option<Context>,
}

impl TemplateObject for AssertMonad {
    fn type_name(&self) -> &str {
        "Template::Monad::Assert"
    }

    fn call_method(&self, method: &str, args: &[Value], _ctx: CallContext) -> MethodResult {
        let stash = self.context.as_ref().map(|c| c.stash()).unwrap_or_default();
        let item_val = Value::Str(method.to_string());
        let res = if let Some(ref target) = self.target {
            stash.dotop(target, &item_val, args, false)
        } else {
            stash.dotop_at_root(&item_val, args)
        };
        match res {
            Ok(v) => {
                if !v.is_defined() {
                    MethodResult::Err(ControlFlow::Exception(TemplateException::new(
                        "assert",
                        format!("undefined value for {}", method),
                    )))
                } else {
                    MethodResult::Ok(v)
                }
            }
            Err(cf) => MethodResult::Err(cf),
        }
    }
}

fn load_assert_plugin(context: &Context) -> Value {
    GLOBAL_VMETHODS.with(|g| {
        let mut vm = g.borrow_mut();
        vm.define_vmethod("scalar", "assert", assert_vmethod);
        vm.define_vmethod("hash", "assert", assert_vmethod);
        vm.define_vmethod("list", "assert", assert_vmethod);
    });
    Value::Object(Rc::new(AssertMonad {
        target: None,
        context: Some(context.clone()),
    }))
}

// ----------------------------------------------------------------------------
// CGI Plugin (minimal: query-string parsing, param(), params())
// ----------------------------------------------------------------------------

fn cgi_unescape(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => {
                let h = std::str::from_utf8(&b[i + 1..i + 3])
                    .ok()
                    .and_then(|h| u8::from_str_radix(h, 16).ok());
                if let Some(v) = h {
                    out.push(v);
                    i += 2;
                } else {
                    out.push(b'%');
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

struct CgiObject {
    params: Vec<(String, Vec<String>)>,
}

// Perl method calls in list context return a list; TT collapses a one-element
// list to the element itself and a longer one to a list reference.
fn cgi_list_result(vals: Vec<String>) -> Value {
    match vals.len() {
        0 => Value::Undefined,
        1 => Value::Str(vals.into_iter().next().unwrap()),
        _ => Value::from_list(vals.into_iter().map(Value::Str).collect()),
    }
}

impl TemplateObject for CgiObject {
    fn type_name(&self) -> &str {
        "CGI"
    }

    // CGI objects are blessed hashes in Perl, so hash vmethods (including
    // the `scalar` vmethod) apply to them.
    fn as_hash(&self) -> Option<Rc<RefCell<HashMap<String, Value>>>> {
        Some(Rc::new(RefCell::new(HashMap::new())))
    }

    fn call_method(&self, method: &str, args: &[Value], ctx: CallContext) -> MethodResult {
        match method {
            "param" => {
                if let Some(name) = args.first().filter(|v| v.is_defined()) {
                    let name = name.to_tt_string();
                    match self.params.iter().find(|(k, _)| *k == name) {
                        None => MethodResult::Ok(Value::Undefined),
                        Some((_, vals)) => {
                            if matches!(ctx, CallContext::List) {
                                MethodResult::Ok(cgi_list_result(vals.clone()))
                            } else {
                                MethodResult::Ok(Value::Str(vals[0].clone()))
                            }
                        }
                    }
                } else {
                    MethodResult::Ok(cgi_list_result(
                        self.params.iter().map(|(k, _)| k.clone()).collect(),
                    ))
                }
            }
            "params" => {
                let mut m = HashMap::new();
                for (k, vals) in &self.params {
                    let v = if vals.len() > 1 {
                        Value::from_list(vals.iter().map(|v| Value::Str(v.clone())).collect())
                    } else {
                        Value::Str(vals.first().cloned().unwrap_or_default())
                    };
                    m.insert(k.clone(), v);
                }
                MethodResult::Ok(Value::from_hash(m))
            }
            _ => MethodResult::NotFound,
        }
    }
}

fn load_cgi_plugin(args: &[Value]) -> Value {
    let query = args
        .first()
        .filter(|v| v.is_defined())
        .map(|v| v.to_tt_string())
        .unwrap_or_default();
    let mut params: Vec<(String, Vec<String>)> = Vec::new();
    for pair in query.split(['&', ';']).filter(|p| !p.is_empty()) {
        let (k, v) = match pair.find('=') {
            Some(i) => (&pair[..i], &pair[i + 1..]),
            None => (pair, ""),
        };
        let (k, v) = (cgi_unescape(k), cgi_unescape(v));
        match params.iter_mut().find(|(n, _)| *n == k) {
            Some((_, vals)) => vals.push(v),
            None => params.push((k, vec![v])),
        }
    }
    Value::Object(Rc::new(CgiObject { params }))
}

// ============================================================================
// 2. Scalar Plugin
// ============================================================================

fn scalar_vmethod(val: &Value, _args: &[Value]) -> Result<Value, ControlFlow> {
    Ok(Value::Object(Rc::new(ScalarMonad {
        target: Some(val.clone()),
        context: None,
    })))
}

struct ScalarMonad {
    target: Option<Value>,
    context: Option<Context>,
}

impl TemplateObject for ScalarMonad {
    fn type_name(&self) -> &str {
        "Template::Monad::Scalar"
    }

    fn call_method(&self, method: &str, args: &[Value], _ctx: CallContext) -> MethodResult {
        if let Some(ref target) = self.target {
            match target {
                Value::Object(obj_rc) => {
                    match obj_rc.call_method(method, args, CallContext::Scalar) {
                        MethodResult::Ok(v) => MethodResult::Ok(v),
                        MethodResult::Err(cf) => MethodResult::Err(cf),
                        MethodResult::NotFound => {
                            MethodResult::Err(ControlFlow::Exception(TemplateException::new(
                                "scalar",
                                format!("invalid object method: {}", method),
                            )))
                        }
                    }
                }
                _ => MethodResult::Err(ControlFlow::Exception(TemplateException::new(
                    "scalar",
                    format!("invalid object method: {}", method),
                ))),
            }
        } else if let Some(ref ctx) = self.context {
            let stash = ctx.stash();
            let raw = stash.get_raw(method);
            match raw {
                None | Some(Value::Undefined) => {
                    MethodResult::Err(ControlFlow::Exception(TemplateException::new(
                        "scalar",
                        format!("undefined value for scalar call: {}", method),
                    )))
                }
                Some(Value::Function(func)) => match func.call(args, CallContext::Scalar) {
                    Ok(v) => MethodResult::Ok(v),
                    Err(cf) => MethodResult::Err(cf),
                },
                Some(other) => MethodResult::Ok(other),
            }
        } else {
            MethodResult::Ok(Value::Undefined)
        }
    }
}

fn load_scalar_plugin(context: &Context) -> Value {
    GLOBAL_VMETHODS.with(|g| {
        let mut vm = g.borrow_mut();
        vm.define_vmethod("hash", "scalar", scalar_vmethod);
        vm.define_vmethod("list", "scalar", scalar_vmethod);
    });
    Value::Object(Rc::new(ScalarMonad {
        target: None,
        context: Some(context.clone()),
    }))
}

// ============================================================================
// 3. Datafile Plugin
// ============================================================================

fn load_datafile_plugin(args: &[Value]) -> Result<Value, TemplateException> {
    let filename = args
        .first()
        .filter(|v| v.is_defined())
        .map(|v| v.to_tt_string())
        .unwrap_or_default();
    if filename.is_empty() {
        return Err(TemplateException::new("datafile", "No filename specified"));
    }

    let mut delim = ":".to_string();
    if let Some(Value::Hash(h_rc)) = args.get(1) {
        if let Some(d) = h_rc.borrow().get("delim").filter(|v| v.is_defined()) {
            let ds = d.to_tt_string();
            if !ds.is_empty() {
                delim = ds;
            }
        }
    }

    let content = fs::read_to_string(&filename)
        .map_err(|e| TemplateException::new("datafile", format!("{}: {}", filename, e)))?;

    let split_pat = format!(r"\s*{}\s*", regex::escape(&delim));
    let split_re = regex::Regex::new(&split_pat).unwrap();

    let mut fields: Option<Vec<String>> = None;
    let mut results = Vec::new();

    for raw_line in content.lines() {
        let line = raw_line.trim_end_matches('\r');
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let trimmed = line.trim_start();
        if fields.is_none() {
            let f: Vec<String> = split_re.split(trimmed).map(|s| s.to_string()).collect();
            fields = Some(f);
        } else if let Some(ref f_names) = fields {
            let parts: Vec<&str> = split_re.splitn(trimmed, f_names.len()).collect();
            let mut record = HashMap::new();
            for (idx, fname) in f_names.iter().enumerate() {
                let val_str = parts.get(idx).copied().unwrap_or("");
                record.insert(fname.clone(), Value::Str(val_str.to_string()));
            }
            results.push(Value::from_hash(record));
        }
    }

    Ok(Value::from_list(results))
}

// ============================================================================
// 4. Date Plugin
// ============================================================================

#[derive(Debug, Clone)]
pub struct DatePlugin {
    pub default_time: Option<Value>,
    pub default_format: String,
    pub default_locale: Option<String>,
    pub default_gmt: bool,
}

impl DatePlugin {
    pub fn format_date(&self, args: &[Value]) -> Result<Value, ControlFlow> {
        let mut pos_args: Vec<Value> = args.to_vec();
        let mut params: HashMap<String, Value> = HashMap::new();
        if let Some(Value::Hash(h_rc)) = pos_args.last().cloned() {
            pos_args.pop();
            params = h_rc.borrow().clone();
        }

        let time_val = pos_args
            .first()
            .cloned()
            .or_else(|| params.get("time").cloned())
            .or_else(|| self.default_time.clone());

        let format_str = pos_args
            .get(1)
            .filter(|v| v.is_defined())
            .map(|v| v.to_tt_string())
            .or_else(|| {
                params
                    .get("format")
                    .filter(|v| v.is_defined())
                    .map(|v| v.to_tt_string())
            })
            .unwrap_or_else(|| self.default_format.clone());

        let locale_opt = pos_args
            .get(2)
            .filter(|v| v.is_defined())
            .map(|v| v.to_tt_string())
            .or_else(|| {
                params
                    .get("locale")
                    .filter(|v| v.is_defined())
                    .map(|v| v.to_tt_string())
            })
            .or_else(|| self.default_locale.clone());

        let gmt = pos_args
            .get(3)
            .map(|v| v.is_truthy())
            .or_else(|| params.get("gmt").map(|v| v.is_truthy()))
            .unwrap_or(self.default_gmt);

        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let time_str = match &time_val {
            None | Some(Value::Undefined) => now_secs.to_string(),
            Some(v) => {
                let s = v.to_tt_string();
                if s.is_empty() || s == "now" {
                    now_secs.to_string()
                } else {
                    s
                }
            }
        };

        let zoned = if !time_str.is_empty() && time_str.chars().all(|c| c.is_ascii_digit()) {
            strftime::from_epoch(time_str.parse::<i64>().unwrap_or(now_secs), gmt)
        } else {
            // Split on non-digits
            let parts: Vec<&str> = time_str
                .split(|c: char| !c.is_ascii_digit())
                .filter(|s| !s.is_empty())
                .collect();
            let bad_date = || {
                ControlFlow::Exception(TemplateException::new(
                    "date",
                    format!(
                        "bad time/date string:  expects 'h:m:s d:m:y'  got: '{}'",
                        time_str
                    ),
                ))
            };
            if parts.len() < 6 {
                return Err(bad_date());
            }
            let n = |i: usize, default: i32| parts[i].parse::<i32>().unwrap_or(default);
            let (sec, min, hour, mday, mon, year) = if parts[0].len() == 4 {
                // Y:M:D H:M:S
                (n(5, 0), n(4, 0), n(3, 0), n(2, 1), n(1, 1), n(0, 1970))
            } else {
                // H:M:S D:M:Y
                (n(2, 0), n(1, 0), n(0, 0), n(3, 1), n(4, 1), n(5, 1970))
            };
            strftime::mktime_local(year, mon, mday, hour, min, sec).ok_or_else(bad_date)?
        };

        let formatted = strftime::strftime(&format_str, &zoned, locale_opt.as_deref());
        Ok(Value::Str(formatted))
    }
}

impl TemplateObject for DatePlugin {
    fn type_name(&self) -> &str {
        "Template::Plugin::Date"
    }

    fn call_method(&self, method: &str, args: &[Value], _ctx: CallContext) -> MethodResult {
        match method {
            "format" => match self.format_date(args) {
                Ok(v) => MethodResult::Ok(v),
                Err(cf) => MethodResult::Err(cf),
            },
            "now" => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                MethodResult::Ok(Value::Int(now))
            }
            _ => MethodResult::NotFound,
        }
    }
}

fn load_date_plugin(args: &[Value]) -> Value {
    let mut default_time = None;
    let mut default_format = "%H:%M:%S %d-%b-%Y".to_string();
    let mut default_locale = None;
    let mut default_gmt = false;

    if let Some(Value::Hash(h_rc)) = args.last() {
        let b = h_rc.borrow();
        if let Some(t) = b.get("time").filter(|v| v.is_defined()) {
            default_time = Some(t.clone());
        }
        if let Some(f) = b.get("format").filter(|v| v.is_defined()) {
            default_format = f.to_tt_string();
        }
        if let Some(l) = b.get("locale").filter(|v| v.is_defined()) {
            default_locale = Some(l.to_tt_string());
        }
        if let Some(g) = b.get("gmt") {
            default_gmt = g.is_truthy();
        }
    }

    Value::Object(Rc::new(DatePlugin {
        default_time,
        default_format,
        default_locale,
        default_gmt,
    }))
}

// ============================================================================
// 5. File & Directory Plugins
// ============================================================================

#[derive(Debug, Clone)]
pub struct FilePlugin {
    pub props: Rc<RefCell<HashMap<String, Value>>>,
}

impl FilePlugin {
    pub fn create(
        path_arg: &str,
        config: &HashMap<String, Value>,
    ) -> Result<Self, TemplateException> {
        if path_arg.is_empty() {
            return Err(TemplateException::new("File", "No file specified"));
        }

        let path = path_arg.trim_end_matches('/').to_string();
        let root = config
            .get("root")
            .filter(|v| v.is_defined())
            .map(|v| v.to_tt_string().trim_end_matches('/').to_string())
            .unwrap_or_default();
        let nostat = config.get("nostat").map(|v| v.is_truthy()).unwrap_or(false);

        let abs = if !root.is_empty() {
            format!("{}/{}", root, path.trim_start_matches('/'))
        } else {
            path.clone()
        };

        let (dir, name) = if let Some(idx) = path.rfind('/') {
            (path[..idx].to_string(), path[idx + 1..].to_string())
        } else {
            (String::new(), path.clone())
        };

        let ext = if let Some(dot_idx) = name.rfind('.') {
            if dot_idx > 0 {
                name[dot_idx + 1..].to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        // home: replace each component of dir (ignoring leading '/') with '..'
        let trimmed_dir = dir.trim_start_matches('/');
        let home = if trimmed_dir.is_empty() {
            String::new()
        } else {
            trimmed_dir
                .split('/')
                .filter(|s| !s.is_empty() && *s != ".")
                .map(|_| "..")
                .collect::<Vec<_>>()
                .join("/")
        };

        let mut props = HashMap::new();
        props.insert("path".to_string(), Value::Str(path));
        props.insert("name".to_string(), Value::Str(name));
        props.insert("root".to_string(), Value::Str(root));
        props.insert("home".to_string(), Value::Str(home));
        props.insert("dir".to_string(), Value::Str(dir));
        props.insert("ext".to_string(), Value::Str(ext));
        props.insert("abs".to_string(), Value::Str(abs.clone()));
        props.insert("user".to_string(), Value::Str(String::new()));
        props.insert("group".to_string(), Value::Str(String::new()));
        props.insert("isdir".to_string(), Value::Str(String::new()));
        props.insert("stat".to_string(), Value::Int(if nostat { 0 } else { 1 }));

        let stat_keys = [
            "dev", "ino", "mode", "nlink", "uid", "gid", "rdev", "size", "atime", "mtime", "ctime",
            "blksize", "blocks",
        ];
        for k in stat_keys {
            props.insert(k.to_string(), Value::Str(String::new()));
        }

        if !nostat {
            let meta = fs::metadata(&abs)
                .map_err(|e| TemplateException::new("File", format!("{}: {}", abs, e)))?;
            props.insert("dev".to_string(), Value::Int(meta.dev() as i64));
            props.insert("ino".to_string(), Value::Int(meta.ino() as i64));
            props.insert("mode".to_string(), Value::Int(meta.mode() as i64));
            props.insert("nlink".to_string(), Value::Int(meta.nlink() as i64));
            props.insert("uid".to_string(), Value::Int(meta.uid() as i64));
            props.insert("gid".to_string(), Value::Int(meta.gid() as i64));
            props.insert("rdev".to_string(), Value::Int(meta.rdev() as i64));
            props.insert("size".to_string(), Value::Int(meta.size() as i64));
            props.insert("atime".to_string(), Value::Int(meta.atime()));
            props.insert("mtime".to_string(), Value::Int(meta.mtime()));
            props.insert("ctime".to_string(), Value::Int(meta.ctime()));
            props.insert("blksize".to_string(), Value::Int(meta.blksize() as i64));
            props.insert("blocks".to_string(), Value::Int(meta.blocks() as i64));
            props.insert(
                "isdir".to_string(),
                Value::Int(if meta.is_dir() { 1 } else { 0 }),
            );
        }

        Ok(Self {
            props: Rc::new(RefCell::new(props)),
        })
    }

    pub fn rel(&self, other: &Value) -> String {
        let target_path = if let Value::Object(obj) = other {
            if let Some(fp) = obj.as_any().downcast_ref::<FilePlugin>() {
                fp.props
                    .borrow()
                    .get("path")
                    .map(|v| v.to_tt_string())
                    .unwrap_or_default()
            } else {
                other.to_tt_string()
            }
        } else {
            other.to_tt_string()
        };

        if target_path.starts_with('/') {
            return target_path;
        }
        let home = self
            .props
            .borrow()
            .get("home")
            .map(|v| v.to_tt_string())
            .unwrap_or_default();
        if home.is_empty() {
            target_path
        } else {
            format!("{}/{}", home, target_path)
        }
    }
}

impl TemplateObject for FilePlugin {
    fn type_name(&self) -> &str {
        "Template::Plugin::File"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_hash(&self) -> Option<Rc<RefCell<HashMap<String, Value>>>> {
        Some(Rc::clone(&self.props))
    }

    fn call_method(&self, method: &str, args: &[Value], _ctx: CallContext) -> MethodResult {
        match method {
            "rel" => {
                let arg = args.first().cloned().unwrap_or(Value::Undefined);
                MethodResult::Ok(Value::Str(self.rel(&arg)))
            }
            "present" => {
                if let Some(Value::Object(v_obj)) = args.first() {
                    if let Some(view) = v_obj.as_any().downcast_ref::<View>() {
                        let is_dir = self
                            .props
                            .borrow()
                            .get("isdir")
                            .map(|v| v.is_truthy())
                            .unwrap_or(false);
                        let tname = if is_dir { "directory" } else { "file" };
                        let self_val = Value::Object(Rc::new(self.clone()));
                        return match view.view_item(tname, Some(self_val), HashMap::new()) {
                            Ok(v) => MethodResult::Ok(v),
                            Err(cf) => MethodResult::Err(cf),
                        };
                    }
                }
                MethodResult::Ok(Value::Str(String::new()))
            }
            _ => {
                if let Some(val) = self.props.borrow().get(method).cloned() {
                    MethodResult::Ok(val)
                } else {
                    MethodResult::NotFound
                }
            }
        }
    }
}

fn load_file_plugin(args: &[Value]) -> Result<Value, TemplateException> {
    let mut pos = args.to_vec();
    let mut cfg = HashMap::new();
    if pos.len() > 1 {
        if let Some(Value::Hash(h_rc)) = pos.last().cloned() {
            pos.pop();
            cfg = h_rc.borrow().clone();
        }
    }
    let path_str = pos.first().map(|v| v.to_tt_string()).unwrap_or_default();
    let fp = FilePlugin::create(&path_str, &cfg)?;
    Ok(Value::Object(Rc::new(fp)))
}

#[derive(Debug, Clone)]
pub struct DirectoryPlugin {
    pub file: FilePlugin,
    pub config: HashMap<String, Value>,
    pub files: Rc<RefCell<Vec<Value>>>,
    pub dirs: Rc<RefCell<Vec<Value>>>,
    pub list: Rc<RefCell<Vec<Value>>>,
    pub by_name: Rc<RefCell<HashMap<String, Value>>>,
}

impl DirectoryPlugin {
    pub fn create(
        path_arg: &str,
        mut config: HashMap<String, Value>,
    ) -> Result<Self, TemplateException> {
        if path_arg.is_empty() {
            return Err(TemplateException::new(
                "Directory",
                "no directory specified",
            ));
        }
        let fp = FilePlugin::create(path_arg, &config)
            .map_err(|e| TemplateException::new("Directory", e.info.to_tt_string()))?;

        let nostat = config.get("nostat").map(|v| v.is_truthy()).unwrap_or(false);
        if !nostat {
            let is_dir = fp
                .props
                .borrow()
                .get("isdir")
                .map(|v| v.is_truthy())
                .unwrap_or(false);
            if !is_dir {
                let abs = fp
                    .props
                    .borrow()
                    .get("abs")
                    .map(|v| v.to_tt_string())
                    .unwrap_or_default();
                return Err(TemplateException::new(
                    "Directory",
                    format!("{}: not a directory", abs),
                ));
            }
        }

        let lazy = config
            .remove("lazy")
            .map(|v| v.is_truthy())
            .unwrap_or(false);
        let dir_plugin = Self {
            file: fp,
            config,
            files: Rc::new(RefCell::new(Vec::new())),
            dirs: Rc::new(RefCell::new(Vec::new())),
            list: Rc::new(RefCell::new(Vec::new())),
            by_name: Rc::new(RefCell::new(HashMap::new())),
        };

        if !lazy {
            dir_plugin.scan(&HashMap::new())?;
        }

        Ok(dir_plugin)
    }

    pub fn scan(&self, scan_cfg: &HashMap<String, Value>) -> Result<(), TemplateException> {
        let mut merged_cfg = self.config.clone();
        for (k, v) in scan_cfg {
            merged_cfg.insert(k.clone(), v.clone());
        }

        let nostat = merged_cfg
            .get("nostat")
            .map(|v| v.is_truthy())
            .unwrap_or(false);
        if nostat {
            return Ok(());
        }

        let recurse = merged_cfg
            .get("recurse")
            .map(|v| v.is_truthy())
            .unwrap_or(false);

        let (abs, rel_path, root) = {
            let b = self.file.props.borrow();
            (
                b.get("abs").map(|v| v.to_tt_string()).unwrap_or_default(),
                b.get("path").map(|v| v.to_tt_string()).unwrap_or_default(),
                b.get("root").map(|v| v.to_tt_string()).unwrap_or_default(),
            )
        };

        let entries = fs::read_dir(&abs)
            .map_err(|e| TemplateException::new("Directory", format!("{}: {}", abs, e)))?;

        let mut names: Vec<String> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| !n.starts_with('.'))
            .collect();
        names.sort();

        let mut files_vec = Vec::new();
        let mut dirs_vec = Vec::new();
        let mut list_vec = Vec::new();
        let mut by_name_map = HashMap::new();

        for name in names {
            let child_rel = format!("{}/{}", rel_path, name);
            let child_abs = format!("{}/{}", abs, name);
            let is_dir = fs::metadata(&child_abs)
                .map(|m| m.is_dir())
                .unwrap_or(false);

            let mut child_cfg = merged_cfg.clone();
            if !root.is_empty() {
                child_cfg.insert("root".to_string(), Value::Str(root.clone()));
            }
            if !recurse {
                child_cfg.insert("lazy".to_string(), Value::Int(1));
            }

            let item_val = if is_dir {
                let dp = DirectoryPlugin::create(&child_rel, child_cfg)?;
                let v = Value::Object(Rc::new(dp));
                dirs_vec.push(v.clone());
                v
            } else {
                let fp = FilePlugin::create(&child_rel, &child_cfg)?;
                let v = Value::Object(Rc::new(fp));
                files_vec.push(v.clone());
                v
            };

            list_vec.push(item_val.clone());
            by_name_map.insert(name, item_val);
        }

        *self.files.borrow_mut() = files_vec;
        *self.dirs.borrow_mut() = dirs_vec;
        *self.list.borrow_mut() = list_vec;
        *self.by_name.borrow_mut() = by_name_map;

        Ok(())
    }
}

impl TemplateObject for DirectoryPlugin {
    fn type_name(&self) -> &str {
        "Template::Plugin::Directory"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_hash(&self) -> Option<Rc<RefCell<HashMap<String, Value>>>> {
        self.file.as_hash()
    }

    fn call_method(&self, method: &str, args: &[Value], ctx: CallContext) -> MethodResult {
        match method {
            "files" => MethodResult::Ok(Value::List(Rc::clone(&self.files))),
            "dirs" => MethodResult::Ok(Value::List(Rc::clone(&self.dirs))),
            "list" => MethodResult::Ok(Value::List(Rc::clone(&self.list))),
            "file" => {
                let name = args.first().map(|v| v.to_tt_string()).unwrap_or_default();
                let found = self
                    .by_name
                    .borrow()
                    .get(&name)
                    .cloned()
                    .unwrap_or(Value::Undefined);
                MethodResult::Ok(found)
            }
            "scan" => {
                let mut cfg = HashMap::new();
                if let Some(Value::Hash(h_rc)) = args.first() {
                    cfg = h_rc.borrow().clone();
                }
                match self.scan(&cfg) {
                    Ok(()) => MethodResult::Ok(Value::Str(String::new())),
                    Err(ex) => MethodResult::Err(ControlFlow::Exception(ex)),
                }
            }
            "present" => {
                if let Some(Value::Object(v_obj)) = args.first() {
                    if let Some(view) = v_obj.as_any().downcast_ref::<View>() {
                        let self_val = Value::Object(Rc::new(self.clone()));
                        return match view.view_item("directory", Some(self_val), HashMap::new()) {
                            Ok(v) => MethodResult::Ok(v),
                            Err(cf) => MethodResult::Err(cf),
                        };
                    }
                }
                MethodResult::Ok(Value::Str(String::new()))
            }
            "content" => {
                let view = match args.first() {
                    Some(v) if v.is_truthy() => v.clone(),
                    _ => return MethodResult::Ok(Value::List(Rc::clone(&self.list))),
                };
                let mut output = String::new();
                let items: Vec<Value> = self.list.borrow().clone();
                for item in items {
                    if let Value::Object(o) = &item {
                        match o.call_method("present", std::slice::from_ref(&view), ctx) {
                            MethodResult::Ok(v) => output.push_str(&v.to_tt_string()),
                            MethodResult::Err(e) => return MethodResult::Err(e),
                            _ => {}
                        }
                    }
                }
                MethodResult::Ok(Value::Str(output))
            }
            _ => self.file.call_method(method, args, ctx),
        }
    }
}

fn load_directory_plugin(args: &[Value], _context: &Context) -> Result<Value, TemplateException> {
    let mut pos = args.to_vec();
    let mut cfg = HashMap::new();
    if pos.len() > 1 {
        if let Some(Value::Hash(h_rc)) = pos.last().cloned() {
            pos.pop();
            cfg = h_rc.borrow().clone();
        }
    }
    let path_str = pos.first().map(|v| v.to_tt_string()).unwrap_or_default();
    let dp = DirectoryPlugin::create(&path_str, cfg)?;
    Ok(Value::Object(Rc::new(dp)))
}

// ============================================================================
// 6. Format Plugin
// ============================================================================

fn make_formatter_fn(format_str: String) -> Value {
    Value::function(move |call_args, _| {
        let effective_args = if call_args.is_empty() {
            vec![Value::Str(String::new())]
        } else {
            call_args.to_vec()
        };
        Ok(Value::Str(perl_sprintf(&format_str, &effective_args)))
    })
}

fn load_format_plugin(args: &[Value]) -> Value {
    if let Some(first) = args.first().filter(|v| v.is_defined()) {
        make_formatter_fn(first.to_tt_string())
    } else {
        Value::function(|call_args, _| {
            let fmt = call_args
                .first()
                .filter(|v| v.is_defined())
                .map(|v| v.to_tt_string())
                .unwrap_or_else(|| "%s".to_string());
            Ok(make_formatter_fn(fmt))
        })
    }
}

// ============================================================================
// 7. HTML Plugin
// ============================================================================

#[derive(Debug, Clone)]
pub struct HtmlPlugin {
    pub sorted: bool,
}

impl HtmlPlugin {
    fn format_attrs(&self, hash_rc: &Rc<RefCell<HashMap<String, Value>>>) -> String {
        let b = hash_rc.borrow();
        let mut keys: Vec<String> = b.keys().cloned().collect();
        if self.sorted {
            keys.sort();
        }
        let mut parts = Vec::with_capacity(keys.len());
        for k in keys {
            if let Some(v) = b.get(&k) {
                let escaped = html_filter(&v.to_tt_string());
                parts.push(format!("{}=\"{}\"", k, escaped));
            }
        }
        parts.join(" ")
    }
}

impl TemplateObject for HtmlPlugin {
    fn type_name(&self) -> &str {
        "Template::Plugin::HTML"
    }

    fn call_method(&self, method: &str, args: &[Value], _ctx: CallContext) -> MethodResult {
        match method {
            "escape" => {
                let text = args.first().map(|v| v.to_tt_string()).unwrap_or_default();
                MethodResult::Ok(Value::Str(html_filter(&text)))
            }
            "url" => {
                let text = args.first().map(|v| v.to_tt_string()).unwrap_or_default();
                MethodResult::Ok(Value::Str(url_filter(&text)))
            }
            "attributes" => {
                if let Some(Value::Hash(h_rc)) = args.first() {
                    MethodResult::Ok(Value::Str(self.format_attrs(h_rc)))
                } else {
                    MethodResult::Ok(Value::Str(String::new()))
                }
            }
            "element" => {
                if let Some(Value::Hash(h_rc)) = args.first() {
                    let b = h_rc.borrow();
                    let mut keys: Vec<String> = b.keys().cloned().collect();
                    keys.sort();
                    let mut out = String::new();
                    for k in keys {
                        if let Some(Value::Hash(attr_rc)) = b.get(&k) {
                            let attrs = self.format_attrs(attr_rc);
                            if attrs.is_empty() {
                                out.push_str(&format!("<{}>", k));
                            } else {
                                out.push_str(&format!("<{} {}>", k, attrs));
                            }
                        } else {
                            out.push_str(&format!("<{}>", k));
                        }
                    }
                    MethodResult::Ok(Value::Str(out))
                } else if let Some(first) = args.first() {
                    let tag = first.to_tt_string();
                    if let Some(Value::Hash(attr_rc)) = args.get(1) {
                        let attrs = self.format_attrs(attr_rc);
                        if attrs.is_empty() {
                            MethodResult::Ok(Value::Str(format!("<{}>", tag)))
                        } else {
                            MethodResult::Ok(Value::Str(format!("<{} {}>", tag, attrs)))
                        }
                    } else {
                        MethodResult::Ok(Value::Str(format!("<{}>", tag)))
                    }
                } else {
                    MethodResult::Ok(Value::Str(String::new()))
                }
            }
            _ => MethodResult::NotFound,
        }
    }
}

fn load_html_plugin(args: &[Value]) -> Value {
    let mut sorted = false;
    if let Some(Value::Hash(h_rc)) = args.last() {
        if let Some(s) = h_rc.borrow().get("sorted") {
            sorted = s.is_truthy();
        }
    }
    Value::Object(Rc::new(HtmlPlugin { sorted }))
}

// ============================================================================
// 8. Image Plugin
// ============================================================================

#[derive(Debug, Clone)]
pub struct ImagePlugin {
    pub name: String,
    pub file: String,
    pub root: String,
    pub alt: Option<String>,
    pub align: Option<String>,
    pub border: Option<String>,
    pub cached_dims: RefCell<Option<(u32, u32)>>,
}

impl ImagePlugin {
    fn get_dims(&self) -> Result<(u32, u32), TemplateException> {
        if let Some(dims) = *self.cached_dims.borrow() {
            return Ok(dims);
        }
        let bytes = fs::read(&self.file)
            .map_err(|e| TemplateException::new("Image", format!("{}: {}", self.file, e)))?;
        let dims = parse_image_dimensions(&bytes).ok_or_else(|| {
            TemplateException::new("Image", format!("{}: unrecognized image format", self.file))
        })?;
        *self.cached_dims.borrow_mut() = Some(dims);
        Ok(dims)
    }

    fn build_attr(&self) -> Result<String, TemplateException> {
        let (w, h) = self.get_dims()?;
        Ok(format!("width=\"{}\" height=\"{}\"", w, h))
    }

    fn build_tag(
        &self,
        extra_opts: Option<&HashMap<String, Value>>,
    ) -> Result<String, TemplateException> {
        let attr = self.build_attr()?;
        let mut tag = format!("<img src=\"{}\" {}", self.name, attr);

        let mut merged: HashMap<String, String> = HashMap::new();
        if let Some(ref a) = self.alt {
            merged.insert("alt".to_string(), a.clone());
        }
        if let Some(ref al) = self.align {
            merged.insert("align".to_string(), al.clone());
        }
        if let Some(ref b) = self.border {
            merged.insert("border".to_string(), b.clone());
        }
        if let Some(opts) = extra_opts {
            for (k, v) in opts {
                merged.insert(k.clone(), v.to_tt_string());
            }
        }
        if !merged.contains_key("alt") {
            merged.insert("alt".to_string(), String::new());
        }

        let mut keys: Vec<String> = merged.keys().cloned().collect();
        keys.sort();
        for k in keys {
            if let Some(val) = merged.get(&k) {
                tag.push_str(&format!(" {}=\"{}\"", k, val));
            }
        }
        tag.push_str(" />");
        Ok(tag)
    }
}

fn parse_image_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    // 1. PNG: 89 50 4E 47 0D 0A 1A 0A + 4-byte len + "IHDR" + 4-byte width + 4-byte height
    if bytes.len() >= 24 && &bytes[0..8] == b"\x89PNG\r\n\x1a\n" {
        let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        return Some((w, h));
    }
    // 2. GIF: "GIF87a" or "GIF89a" + 2-byte LE width + 2-byte LE height
    if bytes.len() >= 10 && (&bytes[0..6] == b"GIF87a" || &bytes[0..6] == b"GIF89a") {
        let w = u16::from_le_bytes([bytes[6], bytes[7]]) as u32;
        let h = u16::from_le_bytes([bytes[8], bytes[9]]) as u32;
        return Some((w, h));
    }
    // 3. JPEG: FF D8 ...
    if bytes.len() >= 4 && bytes[0] == 0xFF && bytes[1] == 0xD8 {
        let mut i = 2;
        while i + 8 < bytes.len() {
            if bytes[i] != 0xFF {
                i += 1;
                continue;
            }
            let marker = bytes[i + 1];
            if marker == 0xFF {
                i += 1;
                continue;
            }
            if matches!(
                marker,
                0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF
            ) {
                let h = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]) as u32;
                let w = u16::from_be_bytes([bytes[i + 7], bytes[i + 8]]) as u32;
                return Some((w, h));
            }
            let seg_len = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
            if seg_len < 2 {
                break;
            }
            i += 2 + seg_len;
        }
    }
    None
}

impl TemplateObject for ImagePlugin {
    fn type_name(&self) -> &str {
        "Template::Plugin::Image"
    }

    fn call_method(&self, method: &str, args: &[Value], _ctx: CallContext) -> MethodResult {
        match method {
            "name" => MethodResult::Ok(Value::Str(self.name.clone())),
            "file" => MethodResult::Ok(Value::Str(self.file.clone())),
            "root" => MethodResult::Ok(Value::Str(self.root.clone())),
            "width" => match self.get_dims() {
                Ok((w, _)) => MethodResult::Ok(Value::Int(w as i64)),
                Err(ex) => MethodResult::Err(ControlFlow::Exception(ex)),
            },
            "height" => match self.get_dims() {
                Ok((_, h)) => MethodResult::Ok(Value::Int(h as i64)),
                Err(ex) => MethodResult::Err(ControlFlow::Exception(ex)),
            },
            "size" => match self.get_dims() {
                Ok((w, h)) => MethodResult::Ok(Value::from_list(vec![
                    Value::Int(w as i64),
                    Value::Int(h as i64),
                ])),
                Err(ex) => MethodResult::Err(ControlFlow::Exception(ex)),
            },
            "attr" => match self.build_attr() {
                Ok(s) => MethodResult::Ok(Value::Str(s)),
                Err(ex) => MethodResult::Err(ControlFlow::Exception(ex)),
            },
            "tag" => {
                let extra = args.first().and_then(|v| match v {
                    Value::Hash(h_rc) => Some(h_rc.borrow().clone()),
                    _ => None,
                });
                match self.build_tag(extra.as_ref()) {
                    Ok(s) => MethodResult::Ok(Value::Str(s)),
                    Err(ex) => MethodResult::Err(ControlFlow::Exception(ex)),
                }
            }
            "modtime" => {
                let mtime = fs::metadata(&self.file)
                    .ok()
                    .map(|m| m.mtime())
                    .unwrap_or(0);
                MethodResult::Ok(Value::Int(mtime))
            }
            _ => MethodResult::NotFound,
        }
    }
}

fn load_image_plugin(args: &[Value]) -> Result<Value, TemplateException> {
    let mut pos = args.to_vec();
    let mut cfg = HashMap::new();
    if let Some(Value::Hash(h_rc)) = pos.last().cloned() {
        pos.pop();
        cfg = h_rc.borrow().clone();
    }

    let name = pos
        .first()
        .filter(|v| v.is_defined())
        .map(|v| v.to_tt_string())
        .or_else(|| {
            cfg.get("name")
                .or_else(|| cfg.get("file"))
                .filter(|v| v.is_defined())
                .map(|v| v.to_tt_string())
        })
        .unwrap_or_default();

    if name.is_empty() {
        return Err(TemplateException::new("Image", "no image file specified"));
    }

    let root = cfg
        .get("root")
        .filter(|v| v.is_defined())
        .map(|v| v.to_tt_string())
        .unwrap_or_default();

    let file = if !root.is_empty() {
        Path::new(&root).join(&name).to_string_lossy().into_owned()
    } else {
        cfg.get("file")
            .filter(|v| v.is_defined())
            .map(|v| v.to_tt_string())
            .unwrap_or_else(|| name.clone())
    };

    let alt = cfg
        .get("alt")
        .filter(|v| v.is_defined())
        .map(|v| v.to_tt_string());
    let align = cfg
        .get("align")
        .filter(|v| v.is_defined())
        .map(|v| v.to_tt_string());
    let border = cfg
        .get("border")
        .filter(|v| v.is_defined())
        .map(|v| v.to_tt_string());

    Ok(Value::Object(Rc::new(ImagePlugin {
        name,
        file,
        root,
        alt,
        align,
        border,
        cached_dims: RefCell::new(None),
    })))
}

// ============================================================================
// 9. Iterator Plugin
// ============================================================================

fn load_iterator_plugin(args: &[Value]) -> Value {
    let data = args.first().cloned().unwrap_or(Value::Undefined);
    let iter = TemplateIterator::new(&data);
    Value::Object(Rc::new(iter))
}

// ============================================================================
// 10. Math Plugin
// ============================================================================

thread_local! {
    /// Math.rand/srand state (splitmix64), seeded from the clock like Perl.
    static RNG_STATE: std::cell::Cell<u64> = std::cell::Cell::new(time_seed());
}

fn time_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

/// Uniform float in [0, 1).
fn next_rand() -> f64 {
    let x = RNG_STATE.with(|s| {
        let v = s.get().wrapping_add(0x9E37_79B9_7F4A_7C15);
        s.set(v);
        v
    });
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

#[derive(Debug, Clone)]
pub struct MathPlugin;

impl TemplateObject for MathPlugin {
    fn type_name(&self) -> &str {
        "Template::Plugin::Math"
    }

    fn call_method(&self, method: &str, args: &[Value], _ctx: CallContext) -> MethodResult {
        let a0 = args.first().map(|v| v.to_number()).unwrap_or(0.0);
        match method {
            "abs" => MethodResult::Ok(Value::Str(format_perl_float(a0.abs()))),
            "atan2" => {
                let a1 = args.get(1).map(|v| v.to_number()).unwrap_or(0.0);
                MethodResult::Ok(Value::Str(format_perl_float(a0.atan2(a1))))
            }
            "cos" => MethodResult::Ok(Value::Str(format_perl_float(a0.cos()))),
            "exp" => MethodResult::Ok(Value::Str(format_perl_float(a0.exp()))),
            "hex" => {
                let s = args.first().map(|v| v.to_tt_string()).unwrap_or_default();
                let clean = s.trim().trim_start_matches("0x").trim_start_matches("0X");
                let n = i64::from_str_radix(clean, 16).unwrap_or(0);
                MethodResult::Ok(Value::Int(n))
            }
            "int" => MethodResult::Ok(Value::Int(a0.trunc() as i64)),
            "log" => MethodResult::Ok(Value::Str(format_perl_float(a0.ln()))),
            "oct" => {
                let s = args.first().map(|v| v.to_tt_string()).unwrap_or_default();
                let clean = s.trim().trim_start_matches('0');
                let n = if clean.starts_with('x') || clean.starts_with('X') {
                    i64::from_str_radix(&clean[1..], 16).unwrap_or(0)
                } else if clean.starts_with('b') || clean.starts_with('B') {
                    i64::from_str_radix(&clean[1..], 2).unwrap_or(0)
                } else {
                    i64::from_str_radix(clean, 8).unwrap_or(0)
                };
                MethodResult::Ok(Value::Int(n))
            }
            "sin" => MethodResult::Ok(Value::Str(format_perl_float(a0.sin()))),
            "sqrt" => MethodResult::Ok(Value::Str(format_perl_float(a0.sqrt()))),
            "srand" => {
                // Perl's srand returns the seed it used.
                let seed = match args.first() {
                    Some(v) if v.is_defined() => v.to_int() as u64,
                    _ => time_seed(),
                };
                RNG_STATE.with(|s| s.set(seed));
                MethodResult::Ok(Value::Int(seed as i64))
            }
            "rand" => {
                let scale = if args.is_empty() { 1.0 } else { a0 };
                let r = next_rand();
                MethodResult::Ok(Value::Str(format_perl_float(r * scale)))
            }
            _ => MethodResult::NotFound,
        }
    }
}

fn load_math_plugin() -> Value {
    Value::Object(Rc::new(MathPlugin))
}

// ============================================================================
// 11. String Plugin
// ============================================================================

/// Pending `filter(name, args)` calls recorded by the String plugin.
type FilterChain = Vec<(String, Vec<Value>)>;

#[derive(Clone)]
pub struct StringPlugin {
    pub text: Rc<RefCell<String>>,
    pub filters: Rc<RefCell<FilterChain>>,
    pub context: Context,
}

impl std::fmt::Debug for StringPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StringPlugin")
            .field("text", &self.text.borrow())
            .finish()
    }
}

impl StringPlugin {
    pub fn from_args(args: &[Value], context: Context) -> Self {
        let mut pos = args.to_vec();
        let mut cfg = HashMap::new();
        if let Some(Value::Hash(h_rc)) = pos.last().cloned() {
            pos.pop();
            cfg = h_rc.borrow().clone();
        }
        let text = if let Some(t) = cfg.get("text").filter(|v| v.is_defined()) {
            t.to_tt_string()
        } else if let Some(first) = pos.first().filter(|v| v.is_defined()) {
            first.to_tt_string()
        } else {
            String::new()
        };

        let sp = Self {
            text: Rc::new(RefCell::new(text)),
            filters: Rc::new(RefCell::new(Vec::new())),
            context,
        };

        if let Some(f) = cfg.get("filter").or_else(|| cfg.get("filters")) {
            sp.add_output_filter(f);
        }

        sp
    }

    pub fn add_output_filter(&self, filter_val: &Value) {
        let items: Vec<Value> = match filter_val {
            Value::Hash(h_rc) => {
                let mut v = Vec::new();
                for (k, val) in h_rc.borrow().iter() {
                    v.push(Value::Str(k.clone()));
                    v.push(val.clone());
                }
                v
            }
            Value::List(l_rc) => l_rc.borrow().clone(),
            other => {
                let s = other.to_tt_string();
                let re = regex::Regex::new(r"\s*\W+\s*").unwrap();
                re.split(&s)
                    .filter(|p| !p.is_empty())
                    .map(|p| Value::Str(p.to_string()))
                    .collect()
            }
        };

        let mut i = 0;
        let mut filters_b = self.filters.borrow_mut();
        while i < items.len() {
            let name = items[i].to_tt_string();
            i += 1;
            let mut f_args = Vec::new();
            if i < items.len() {
                let next_is_arg = items[i].is_ref() || items[i].to_tt_string().is_empty();
                if next_is_arg {
                    let arg_val = items[i].clone();
                    i += 1;
                    if arg_val.is_truthy() {
                        if let Value::List(l_rc) = arg_val {
                            f_args = l_rc.borrow().clone();
                        } else {
                            f_args = vec![arg_val];
                        }
                    }
                }
            }
            filters_b.push((name, f_args));
        }
    }

    pub fn rendered_text(&self) -> Result<String, ControlFlow> {
        let mut t = self.text.borrow().clone();
        let filters = self.filters.borrow().clone();
        for (fname, fargs) in filters {
            let filter_fn = self.context.get_filter(&fname, &fargs)?;
            t = filter_fn(&t)?;
        }
        Ok(t)
    }

    fn self_val(&self) -> Value {
        Value::Object(Rc::new(self.clone()))
    }
}

impl TemplateObject for StringPlugin {
    fn type_name(&self) -> &str {
        "Template::Plugin::String"
    }

    fn to_tt_string(&self) -> String {
        self.rendered_text()
            .unwrap_or_else(|_| self.text.borrow().clone())
    }

    fn call_method(&self, method: &str, args: &[Value], _ctx: CallContext) -> MethodResult {
        match method {
            "text" => match self.rendered_text() {
                Ok(s) => MethodResult::Ok(Value::Str(s)),
                Err(cf) => MethodResult::Err(cf),
            },
            "new" => {
                let sp = StringPlugin::from_args(args, self.context.clone());
                MethodResult::Ok(Value::Object(Rc::new(sp)))
            }
            "copy" => {
                let sp = StringPlugin {
                    text: Rc::new(RefCell::new(self.text.borrow().clone())),
                    filters: Rc::new(RefCell::new(Vec::new())),
                    context: self.context.clone(),
                };
                MethodResult::Ok(Value::Object(Rc::new(sp)))
            }
            "output_filter" => {
                if let Some(first) = args.first() {
                    self.add_output_filter(first);
                }
                MethodResult::Ok(Value::Str(String::new()))
            }
            "push" | "append" => {
                let mut t = self.text.borrow_mut();
                for a in args {
                    t.push_str(&a.to_tt_string());
                }
                drop(t);
                MethodResult::Ok(self.self_val())
            }
            "unshift" | "prepend" => {
                let mut prefix = String::new();
                for a in args {
                    prefix.push_str(&a.to_tt_string());
                }
                let mut t = self.text.borrow_mut();
                *t = format!("{}{}", prefix, *t);
                drop(t);
                MethodResult::Ok(self.self_val())
            }
            "pop" => {
                if let Some(strip) = args.first().filter(|v| v.is_truthy()) {
                    let pat = format!("{}$", strip.to_tt_string());
                    if let Ok(re) = regex::Regex::new(&pat) {
                        let mut t = self.text.borrow_mut();
                        *t = re.replace(&t, "").into_owned();
                    }
                }
                MethodResult::Ok(self.self_val())
            }
            "shift" => {
                if let Some(strip) = args.first().filter(|v| v.is_truthy()) {
                    let pat = format!("^{}", strip.to_tt_string());
                    if let Ok(re) = regex::Regex::new(&pat) {
                        let mut t = self.text.borrow_mut();
                        *t = re.replace(&t, "").into_owned();
                    }
                }
                MethodResult::Ok(self.self_val())
            }
            "center" | "centre" => {
                let width = args
                    .first()
                    .map(|v| v.to_int().max(0) as usize)
                    .unwrap_or(0);
                let mut t = self.text.borrow_mut();
                let len = t.chars().count();
                if len < width {
                    let lpad = (width - len) / 2;
                    let rpad = width - len - lpad;
                    *t = format!("{}{}{}", " ".repeat(lpad), *t, " ".repeat(rpad));
                }
                drop(t);
                MethodResult::Ok(self.self_val())
            }
            "left" => {
                let width = args
                    .first()
                    .map(|v| v.to_int().max(0) as usize)
                    .unwrap_or(0);
                let mut t = self.text.borrow_mut();
                let len = t.chars().count();
                if len < width {
                    t.push_str(&" ".repeat(width - len));
                }
                drop(t);
                MethodResult::Ok(self.self_val())
            }
            "right" => {
                let width = args
                    .first()
                    .map(|v| v.to_int().max(0) as usize)
                    .unwrap_or(0);
                let mut t = self.text.borrow_mut();
                let len = t.chars().count();
                if len < width {
                    *t = format!("{}{}", " ".repeat(width - len), *t);
                }
                drop(t);
                MethodResult::Ok(self.self_val())
            }
            "format" => {
                let fmt = args
                    .first()
                    .filter(|v| v.is_defined())
                    .map(|v| v.to_tt_string())
                    .unwrap_or_else(|| "%s".to_string());
                let mut t = self.text.borrow_mut();
                *t = perl_sprintf(&fmt, &[Value::Str(t.clone())]);
                drop(t);
                MethodResult::Ok(self.self_val())
            }
            "filter" => {
                let fname = args.first().map(|v| v.to_tt_string()).unwrap_or_default();
                let fargs = if args.len() > 1 { &args[1..] } else { &[] };
                match self.context.get_filter(&fname, fargs) {
                    Ok(ffn) => match ffn(&self.text.borrow()) {
                        Ok(s) => MethodResult::Ok(Value::Str(s)),
                        Err(cf) => MethodResult::Err(cf),
                    },
                    Err(cf) => MethodResult::Err(cf),
                }
            }
            "upper" => {
                let mut t = self.text.borrow_mut();
                *t = t.to_uppercase();
                drop(t);
                MethodResult::Ok(self.self_val())
            }
            "lower" => {
                let mut t = self.text.borrow_mut();
                *t = t.to_lowercase();
                drop(t);
                MethodResult::Ok(self.self_val())
            }
            "capital" => {
                let mut t = self.text.borrow_mut();
                let mut chars = t.chars();
                if let Some(first) = chars.next() {
                    *t = first.to_uppercase().collect::<String>() + chars.as_str();
                }
                drop(t);
                MethodResult::Ok(self.self_val())
            }
            "chop" => {
                let mut t = self.text.borrow_mut();
                t.pop();
                drop(t);
                MethodResult::Ok(self.self_val())
            }
            "chomp" => {
                let mut t = self.text.borrow_mut();
                if t.ends_with('\n') {
                    t.pop();
                }
                drop(t);
                MethodResult::Ok(self.self_val())
            }
            "trim" => {
                let mut t = self.text.borrow_mut();
                *t = t.trim().to_string();
                drop(t);
                MethodResult::Ok(self.self_val())
            }
            "collapse" => {
                let mut t = self.text.borrow_mut();
                let trimmed = t.trim();
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
                *t = out;
                drop(t);
                MethodResult::Ok(self.self_val())
            }
            "length" => {
                let len = self.text.borrow().chars().count() as i64;
                MethodResult::Ok(Value::Int(len))
            }
            "truncate" => {
                let Some(len_val) = args.first().filter(|v| v.is_defined()) else {
                    return MethodResult::Ok(self.self_val());
                };
                let max_len = len_val.to_int().max(0) as usize;
                let suffix = args
                    .get(1)
                    .filter(|v| v.is_defined())
                    .map(|v| v.to_tt_string())
                    .unwrap_or_default();
                let mut t = self.text.borrow_mut();
                let chars: Vec<char> = t.chars().collect();
                if chars.len() > max_len {
                    let suf_len = suffix.chars().count();
                    let keep = max_len.saturating_sub(suf_len);
                    let prefix: String = chars[..keep].iter().collect();
                    *t = format!("{}{}", prefix, suffix);
                }
                drop(t);
                MethodResult::Ok(self.self_val())
            }
            "substr" => {
                let cur_val = Value::Str(self.text.borrow().clone());
                if args.len() >= 3 && args[2].is_defined() {
                    let offset = args[0].to_int();
                    let length = args[1].to_int().max(0) as usize;
                    let replacement = args[2].to_tt_string();
                    let removed = vmethod_scalar_substr(&cur_val, &args[..2])
                        .unwrap_or(Value::Str(String::new()));
                    let mut t = self.text.borrow_mut();
                    let chars: Vec<char> = t.chars().collect();
                    let total = chars.len() as i64;
                    let start =
                        (if offset < 0 { total + offset } else { offset }).clamp(0, total) as usize;
                    let end = (start + length).min(chars.len());
                    let before: String = chars[..start].iter().collect();
                    let after: String = chars[end..].iter().collect();
                    *t = format!("{}{}{}", before, replacement, after);
                    MethodResult::Ok(removed)
                } else {
                    match vmethod_scalar_substr(&cur_val, args) {
                        Ok(v) => MethodResult::Ok(v),
                        Err(cf) => MethodResult::Err(cf),
                    }
                }
            }
            "repeat" => {
                if let Some(n_val) = args.first().filter(|v| v.is_defined()) {
                    let n = n_val.to_int().max(0) as usize;
                    let mut t = self.text.borrow_mut();
                    *t = t.repeat(n);
                }
                MethodResult::Ok(self.self_val())
            }
            "replace" => {
                if let Some(s_val) = args.first().filter(|v| v.is_defined()) {
                    let search = s_val.to_tt_string();
                    let replace = args
                        .get(1)
                        .filter(|v| v.is_defined())
                        .map(|v| v.to_tt_string())
                        .unwrap_or_default();
                    let mut t = self.text.borrow_mut();
                    if let Ok(re) = regex::Regex::new(&search) {
                        *t = re.replace_all(&t, replace.as_str()).into_owned();
                    } else {
                        *t = t.replace(&search, &replace);
                    }
                }
                MethodResult::Ok(self.self_val())
            }
            "remove" => {
                let search = args
                    .first()
                    .filter(|v| v.is_defined())
                    .map(|v| v.to_tt_string())
                    .unwrap_or_default();
                let mut t = self.text.borrow_mut();
                if let Ok(re) = regex::Regex::new(&search) {
                    *t = re.replace_all(&t, "").into_owned();
                } else {
                    *t = t.replace(&search, "");
                }
                drop(t);
                MethodResult::Ok(self.self_val())
            }
            "split" => {
                let cur_val = Value::Str(self.text.borrow().clone());
                match vmethod_scalar_split(&cur_val, args) {
                    Ok(v) => MethodResult::Ok(v),
                    Err(cf) => MethodResult::Err(cf),
                }
            }
            "search" => {
                let pat = args.first().map(|v| v.to_tt_string()).unwrap_or_default();
                let matched = regex::Regex::new(&pat)
                    .map(|re| re.is_match(&self.text.borrow()))
                    .unwrap_or(false);
                MethodResult::Ok(Value::Int(if matched { 1 } else { 0 }))
            }
            "equals" => {
                let cmp = args.first().map(|v| v.to_tt_string()).unwrap_or_default();
                let eq = *self.text.borrow() == cmp;
                MethodResult::Ok(Value::Int(if eq { 1 } else { 0 }))
            }
            _ => MethodResult::NotFound,
        }
    }
}

fn load_string_plugin(args: &[Value], context: &Context) -> Result<Value, TemplateException> {
    let sp = StringPlugin::from_args(args, context.clone());
    Ok(Value::Object(Rc::new(sp)))
}

// ============================================================================
// 12. Table Plugin
// ============================================================================

#[derive(Debug, Clone)]
pub struct TablePlugin {
    pub data: Vec<Value>,
    pub size: usize,
    pub nrows: usize,
    pub ncols: usize,
    pub coloff: usize,
    pub overlap: usize,
    pub pad: bool,
}

impl TablePlugin {
    pub fn get_row(&self, r: isize) -> Option<Vec<Value>> {
        if r < 0 || (r as usize) >= self.nrows {
            return None;
        }
        let mut set = Vec::with_capacity(self.ncols);
        let mut idx = r as usize;
        for _ in 0..self.ncols {
            if idx < self.size {
                set.push(self.data[idx].clone());
            } else if self.pad {
                set.push(Value::Undefined);
            }
            idx += self.coloff;
        }
        Some(set)
    }

    pub fn get_col(&self, c: isize) -> Option<Vec<Value>> {
        if c < 0 || (c as usize) >= self.ncols {
            return None;
        }
        let start = self.coloff * (c as usize);
        if start >= self.size {
            return Some(Vec::new());
        }
        let mut end = if self.nrows > 0 {
            start + self.nrows - 1
        } else {
            start
        };
        let mut blanks = 0;
        if end >= self.size {
            blanks = (end - self.size) + 1;
            end = self.size - 1;
        }
        let mut col_items: Vec<Value> = self.data[start..=end].to_vec();
        if self.pad && blanks > 0 {
            col_items.extend(std::iter::repeat_n(Value::Undefined, blanks));
        }
        Some(col_items)
    }

    pub fn get_rows(&self) -> Vec<Value> {
        (0..self.nrows)
            .filter_map(|r| self.get_row(r as isize).map(Value::from_list))
            .collect()
    }

    pub fn get_cols(&self) -> Vec<Value> {
        (0..self.ncols)
            .filter_map(|c| self.get_col(c as isize))
            // Perl's `return ()` inside map contributes nothing to the result
            .filter(|col| !col.is_empty())
            .map(Value::from_list)
            .collect()
    }
}

impl TemplateObject for TablePlugin {
    fn type_name(&self) -> &str {
        "Template::Plugin::Table"
    }

    fn call_method(&self, method: &str, args: &[Value], _ctx: CallContext) -> MethodResult {
        match method {
            "row" => {
                if let Some(first) = args.first().filter(|v| v.is_defined()) {
                    let r = first.to_int() as isize;
                    match self.get_row(r) {
                        Some(list) => MethodResult::Ok(Value::from_list(list)),
                        None => MethodResult::Ok(Value::Undefined),
                    }
                } else {
                    MethodResult::Ok(Value::from_list(self.get_rows()))
                }
            }
            "col" => {
                if let Some(first) = args.first().filter(|v| v.is_defined()) {
                    let c = first.to_int() as isize;
                    match self.get_col(c) {
                        Some(list) => MethodResult::Ok(Value::from_list(list)),
                        None => MethodResult::Ok(Value::Undefined),
                    }
                } else {
                    MethodResult::Ok(Value::from_list(self.get_cols()))
                }
            }
            "rows" => MethodResult::Ok(Value::from_list(self.get_rows())),
            "cols" => MethodResult::Ok(Value::from_list(self.get_cols())),
            _ => {
                let upper = format!("_{}", method.to_ascii_uppercase());
                match upper.as_str() {
                    "_NROWS" => MethodResult::Ok(Value::Int(self.nrows as i64)),
                    "_NCOLS" => MethodResult::Ok(Value::Int(self.ncols as i64)),
                    "_SIZE" => MethodResult::Ok(Value::Int(self.size as i64)),
                    "_OVERLAP" => MethodResult::Ok(Value::Int(self.overlap as i64)),
                    "_PAD" => MethodResult::Ok(Value::Int(if self.pad { 1 } else { 0 })),
                    _ => MethodResult::NotFound,
                }
            }
        }
    }
}

fn load_table_plugin(args: &[Value]) -> Result<Value, TemplateException> {
    let first = args.first().cloned().unwrap_or(Value::Undefined);
    let data_vec: Vec<Value> = match first {
        Value::List(l_rc) => l_rc.borrow().clone(),
        Value::Object(obj) => {
            if let Some(iter) = obj.as_any().downcast_ref::<TemplateIterator>() {
                let (all_val, _) = iter.get_all();
                match all_val {
                    Value::List(l_rc) => l_rc.borrow().clone(),
                    _ => Vec::new(),
                }
            } else if let Some(l_rc) = obj.as_list() {
                l_rc.borrow().clone()
            } else {
                return Err(TemplateException::new(
                    "Table",
                    "invalid table data, expecting a list",
                ));
            }
        }
        _ => {
            return Err(TemplateException::new(
                "Table",
                "invalid table data, expecting a list",
            ));
        }
    };

    let mut params = HashMap::new();
    if let Some(Value::Hash(h_rc)) = args.get(1) {
        for (k, v) in h_rc.borrow().iter() {
            params.insert(k.to_ascii_uppercase(), v.clone());
        }
    }

    let size = data_vec.len();
    let overlap = params
        .get("OVERLAP")
        .map(|v| v.to_int().max(0) as usize)
        .unwrap_or(0);
    let pad = params.get("PAD").map(|v| v.is_truthy()).unwrap_or(true);

    let (nrows, ncols, coloff) = if let Some(r_val) = params.get("ROWS").filter(|v| v.is_truthy()) {
        let r = r_val.to_int().max(1) as usize;
        if size < r {
            (size, 1, 0)
        } else {
            let co = r.saturating_sub(overlap).max(1);
            let c = (size / co) + if (size % co) > overlap { 1 } else { 0 };
            (r, c, co)
        }
    } else if let Some(c_val) = params.get("COLS").filter(|v| v.is_truthy()) {
        let c = c_val.to_int().max(1) as usize;
        if size < c {
            (1, size, 1)
        } else {
            let co = (size / c) + if (size % c) > overlap { 1 } else { 0 };
            let r = co + overlap;
            (r, c, co)
        }
    } else {
        (size, 1, 0)
    };

    Ok(Value::Object(Rc::new(TablePlugin {
        data: data_vec,
        size,
        nrows,
        ncols,
        coloff,
        overlap,
        pad,
    })))
}

// ============================================================================
// 13. URL Plugin
// ============================================================================

pub fn create_url_callable(base: Option<String>, base_args: HashMap<String, Value>) -> Value {
    let base_url = base.unwrap_or_default();
    Value::function(move |call_args, _| {
        let mut pos = call_args.to_vec();
        let mut add_args = HashMap::new();
        if let Some(Value::Hash(h_rc)) = pos.last().cloned() {
            pos.pop();
            add_args = h_rc.borrow().clone();
        }
        let new_base = pos
            .first()
            .filter(|v| v.is_defined())
            .map(|v| v.to_tt_string())
            .unwrap_or_else(|| base_url.clone());

        let mut combo = base_args.clone();
        for (k, v) in add_args {
            combo.insert(k, v);
        }

        let mut keys: Vec<String> = combo.keys().cloned().collect();
        keys.sort();

        let mut query_parts = Vec::new();
        for k in keys {
            if let Some(val) = combo.get(&k) {
                if !val.is_defined() {
                    continue;
                }
                let esc_k = uri_filter(&k);
                match val {
                    Value::List(l_rc) => {
                        for item in l_rc.borrow().iter() {
                            let esc_v = uri_filter(&item.to_tt_string());
                            query_parts.push(format!("{}={}", esc_k, esc_v));
                        }
                    }
                    other => {
                        let esc_v = uri_filter(&other.to_tt_string());
                        query_parts.push(format!("{}={}", esc_k, esc_v));
                    }
                }
            }
        }

        let joint = get_url_joint();
        let urlargs = query_parts.join(&joint);

        if new_base.is_empty() {
            Ok(Value::Str(urlargs))
        } else if urlargs.is_empty() {
            Ok(Value::Str(new_base))
        } else {
            Ok(Value::Str(format!("{}?{}", new_base, urlargs)))
        }
    })
}

fn load_url_plugin(args: &[Value]) -> Value {
    let mut pos = args.to_vec();
    let mut base_args = HashMap::new();
    if let Some(Value::Hash(h_rc)) = pos.last().cloned() {
        pos.pop();
        base_args = h_rc.borrow().clone();
    }
    let base = pos
        .first()
        .filter(|v| v.is_defined())
        .map(|v| v.to_tt_string());
    create_url_callable(base, base_args)
}

// ============================================================================
// 14. View Plugin
// ============================================================================

fn load_view_plugin(args: &[Value], context: &Context) -> Result<Value, TemplateException> {
    let mut cfg = HashMap::new();
    if let Some(Value::Hash(h_rc)) = args.first() {
        cfg = h_rc.borrow().clone();
    }
    let view = View::new(context.clone(), cfg)?;
    view.seal();
    Ok(Value::Object(Rc::new(view)))
}

// ============================================================================
// 15. Wrap Plugin
// ============================================================================

const WRAP_TABSTOP: usize = 8;

/// `Text::Tabs::expand` for a single string.
fn tabs_expand(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut col = 0;
    for c in text.chars() {
        match c {
            '\t' => {
                let pad = WRAP_TABSTOP - col % WRAP_TABSTOP;
                out.extend(std::iter::repeat_n(' ', pad));
                col += pad;
            }
            '\n' => {
                out.push(c);
                col = 0;
            }
            _ => {
                out.push(c);
                col += 1;
            }
        }
    }
    out
}

/// `Text::Tabs::unexpand`: within each full tabstop-wide chunk of a line, a
/// trailing run of two or more spaces becomes a tab.
fn tabs_unexpand(text: &str) -> String {
    text.split('\n')
        .map(|line| {
            let chars: Vec<char> = tabs_expand(line).chars().collect();
            let full = chars.len() / WRAP_TABSTOP * WRAP_TABSTOP;
            let mut out = String::new();
            for chunk in chars[..full].chunks(WRAP_TABSTOP) {
                let trailing = chunk.iter().rev().take_while(|&&c| c == ' ').count();
                let keep = if trailing >= 2 {
                    chunk.len() - trailing
                } else {
                    chunk.len()
                };
                out.extend(&chunk[..keep]);
                if keep < chunk.len() {
                    out.push('\t');
                }
            }
            out.extend(&chars[full..]);
            out
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Port of `Text::Wrap::wrap($init_tab, $subseq_tab, $text)` with the module
/// defaults Template::Plugin::Wrap uses: `$huge = 'wrap'`, `$unexpand = 1`,
/// `$tabstop = 8`, `$separator = "\n"`, and `$break` matching any single
/// whitespace character (or `\r\n`).
pub fn tt_wrap_text(text: &str, width: usize, init_tab: &str, subseq_tab: &str) -> String {
    let t: Vec<char> = tabs_expand(text).chars().collect();
    let xlen = |s: &str| tabs_expand(s).chars().count();
    let mut columns = width;
    let mut nll = columns as isize - xlen(subseq_tab) as isize - 1;
    if nll <= 0 && !subseq_tab.is_empty() {
        columns = xlen(subseq_tab) + 2;
        nll = 1;
    }
    let nll = nll.max(0) as usize;
    let mut ll = (columns as isize - xlen(init_tab) as isize - 1).max(0) as usize;

    let mut r = String::new();
    let mut lead = init_tab;
    let mut nl = "";
    let mut remainder = String::new();
    let mut pos = 0;
    let emit = |r: &mut String, nl: &str, lead: &str, seg: &[char]| {
        let line: String = nl
            .chars()
            .chain(lead.chars())
            .chain(seg.iter().copied())
            .collect();
        r.push_str(&tabs_unexpand(&line));
    };

    // while ($t !~ /\G(?:$break)*\Z/gc)
    while !t[pos..].iter().all(|c| c.is_whitespace()) {
        // Characters available before the next newline.
        let avail = t[pos..]
            .iter()
            .position(|&c| c == '\n')
            .unwrap_or(t.len() - pos);
        // /\G((?:(?!\n).){0,$ll})($break|\n+|\z)/: longest segment followed
        // by whitespace or end of text.
        let fit = (0..=ll.min(avail))
            .rev()
            .find(|&k| pos + k == t.len() || t[pos + k].is_whitespace());
        if let Some(k) = fit {
            emit(&mut r, nl, lead, &t[pos..pos + k]);
            let d = pos + k;
            let dlen = match (t.get(d), t.get(d + 1)) {
                (None, _) => 0,
                (Some('\r'), Some('\n')) => 2,
                _ => 1,
            };
            remainder = t[d..d + dlen].iter().collect();
            pos = d + dlen;
        } else if ll > 0 && avail >= ll {
            // $huge eq 'wrap': hard-break a word longer than the line.
            emit(&mut r, nl, lead, &t[pos..pos + ll]);
            remainder = "\n".to_string();
            pos += ll;
        } else {
            // Perl gives up here ($columns < 2); emit the rest unwrapped.
            emit(&mut r, nl, lead, &t[pos..]);
            remainder.clear();
            break;
        }
        lead = subseq_tab;
        ll = nll;
        nl = "\n";
    }
    r.push_str(&remainder);
    r
}

fn load_wrap_plugin(context: &Context) -> Value {
    // Also register 'wrap' dynamic filter in context
    context.define_filter(
        "wrap",
        FilterDef::Dynamic(Rc::new(|args| {
            let width = args
                .first()
                .filter(|v| v.is_defined())
                .map(|v| v.to_int().max(1) as usize)
                .unwrap_or(72);
            let init_tab = args
                .get(1)
                .filter(|v| v.is_defined())
                .map(|v| v.to_tt_string())
                .unwrap_or_default();
            let subseq_tab = args
                .get(2)
                .filter(|v| v.is_defined())
                .map(|v| v.to_tt_string())
                .unwrap_or_default();
            let f: FilterFn =
                Rc::new(move |text| Ok(tt_wrap_text(text, width, &init_tab, &subseq_tab)));
            Ok(crate::filters::FilterFactoryOutcome::Ok(f))
        })),
    );

    Value::function(|call_args, _| {
        let text = call_args
            .first()
            .map(|v| v.to_tt_string())
            .unwrap_or_default();
        let width = call_args
            .get(1)
            .filter(|v| v.is_defined())
            .map(|v| v.to_int().max(1) as usize)
            .unwrap_or(72);
        let init_tab = call_args
            .get(2)
            .filter(|v| v.is_defined())
            .map(|v| v.to_tt_string())
            .unwrap_or_default();
        let subseq_tab = call_args
            .get(3)
            .filter(|v| v.is_defined())
            .map(|v| v.to_tt_string())
            .unwrap_or_default();
        Ok(Value::Str(tt_wrap_text(
            &text,
            width,
            &init_tab,
            &subseq_tab,
        )))
    })
}

/// Custom plugin factory trait for user-defined or test-defined plugins.
pub trait PluginFactory {
    fn load(&self, context: &Context, args: &[Value]) -> Result<Value, TemplateException>;
}
