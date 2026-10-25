use std::{
    cell::RefCell,
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    document::Document,
    error::TemplateException,
    parser::{ParseInfo, Parser},
    value::{CallContext, MethodResult, TemplateObject, Value},
};

static GLOBAL_MAX_DIRS: AtomicUsize = AtomicUsize::new(64);
static GLOBAL_STAT_TTL: AtomicUsize = AtomicUsize::new(1);

pub fn set_max_dirs(max: usize) {
    GLOBAL_MAX_DIRS.store(max, Ordering::SeqCst);
}

pub fn get_max_dirs() -> usize {
    GLOBAL_MAX_DIRS.load(Ordering::SeqCst)
}

pub fn set_stat_ttl(ttl: usize) {
    GLOBAL_STAT_TTL.store(ttl, Ordering::SeqCst);
}

pub fn get_stat_ttl() -> usize {
    GLOBAL_STAT_TTL.load(Ordering::SeqCst)
}

#[derive(Clone)]
pub enum IncludePathItem {
    Path(String),
    Generator(Rc<dyn Fn() -> Result<Vec<IncludePathItem>, String>>),
    Object(Rc<dyn TemplateObject>),
}

impl std::fmt::Debug for IncludePathItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IncludePathItem::Path(s) => write!(f, "Path({:?})", s),
            IncludePathItem::Generator(_) => write!(f, "Generator(..)"),
            IncludePathItem::Object(o) => write!(f, "Object({})", o.type_name()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum FetchResult {
    Ok(Rc<Document>),
    Declined,
    Error(TemplateException),
}

#[derive(Debug, Clone)]
pub enum LoadResult {
    Ok(String),
    Declined,
    Error(TemplateException),
}

#[derive(Debug, Clone)]
struct CacheSlot {
    name: String,
    alias: String,
    doc: Rc<Document>,
    load_mtime: u64,
    stat_time: u64,
}

#[derive(Debug, Clone)]
pub struct Provider {
    pub include_path: Vec<IncludePathItem>,
    pub delimiter: String,
    pub compile_dir: Option<String>,
    pub compile_ext: Option<String>,
    pub absolute: bool,
    pub relative: bool,
    pub tolerant: bool,
    pub default: Option<String>,
    pub encoding: Option<String>,
    pub unicode: bool,
    pub cache_size: Option<usize>,
    pub stat_ttl: Option<u64>,
    pub max_dirs: Option<usize>,
    pub parser: Rc<RefCell<Parser>>,
    cache_order: RefCell<Vec<String>>,
    lookup: RefCell<HashMap<String, CacheSlot>>,
    notfound: RefCell<HashMap<String, u64>>,
    pub error_msg: RefCell<String>,
}

#[bon::bon]
impl Provider {
    #[builder(start_fn = builder, finish_fn = build)]
    pub fn build(
        include_path: Option<Vec<IncludePathItem>>,
        include_path_str: Option<String>,
        delimiter: Option<String>,
        compile_dir: Option<String>,
        compile_ext: Option<String>,
        #[builder(default = false)] absolute: bool,
        #[builder(default = false)] relative: bool,
        #[builder(default = false)] tolerant: bool,
        default: Option<String>,
        encoding: Option<String>,
        #[builder(default = true)] unicode: bool,
        cache_size: Option<usize>,
        stat_ttl: Option<u64>,
        max_dirs: Option<usize>,
        parser: Option<Rc<RefCell<Parser>>>,
    ) -> Result<Self, String> {
        let dlim = delimiter.unwrap_or_else(|| ":".to_string());
        let paths = if let Some(items) = include_path {
            items
        } else if let Some(s) = include_path_str {
            s.split(&dlim)
                .map(|p| IncludePathItem::Path(p.to_string()))
                .collect()
        } else {
            vec![IncludePathItem::Path(".".to_string())]
        };

        let adjusted_size = cache_size.map(|sz| if sz == 1 { 2 } else { sz });

        if let Some(ref cdir) = compile_dir {
            if !cdir.is_empty() {
                for item in &paths {
                    if let IncludePathItem::Path(p) = item {
                        let sub = p.trim_start_matches('/');
                        let wdir = Path::new(cdir).join(sub);
                        let _ = fs::create_dir_all(&wdir);
                    }
                }
            }
        }

        let parser_rc = parser.unwrap_or_else(|| Rc::new(RefCell::new(Parser::new())));

        Ok(Self {
            include_path: paths,
            delimiter: dlim,
            compile_dir: compile_dir.filter(|s| !s.is_empty()),
            compile_ext: compile_ext.filter(|s| !s.is_empty()),
            absolute,
            relative,
            tolerant,
            default,
            encoding,
            unicode,
            cache_size: adjusted_size,
            stat_ttl,
            max_dirs,
            parser: parser_rc,
            cache_order: RefCell::new(Vec::new()),
            lookup: RefCell::new(HashMap::new()),
            notfound: RefCell::new(HashMap::new()),
            error_msg: RefCell::new(String::new()),
        })
    }
}

impl Default for Provider {
    fn default() -> Self {
        Self::builder().build().unwrap()
    }
}

fn current_unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn file_mtime(path: &Path) -> Option<u64> {
    fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}

fn set_file_mtime(path: &Path, mtime: u64) {
    let t = UNIX_EPOCH + std::time::Duration::from_secs(mtime);
    if let Ok(f) = fs::File::open(path) {
        let _ = f.set_times(fs::FileTimes::new().set_accessed(t).set_modified(t));
    }
}

fn is_relative_path(name: &str) -> bool {
    // Matches qr[(?:^|/)\.+/]
    if name.starts_with("./") || name.starts_with("../") {
        return true;
    }
    for part in name.split('/') {
        if !part.is_empty() && part.chars().all(|c| c == '.') {
            // Confirm it's followed by '/'
            if name.contains(&format!("/{}/", part)) || name.starts_with(&format!("{}/", part)) {
                return true;
            }
        }
    }
    false
}

impl Provider {
    pub fn error(&self) -> String {
        self.error_msg.borrow().clone()
    }

    pub fn paths(&self) -> Result<Vec<String>, String> {
        let mut queue: Vec<IncludePathItem> = self.include_path.clone();
        let mut out = Vec::new();
        let max_dirs = self.max_dirs.unwrap_or_else(get_max_dirs);
        let mut count = max_dirs;

        while !queue.is_empty() {
            if count == 0 {
                break;
            }
            count -= 1;
            let item = queue.remove(0);
            match item {
                IncludePathItem::Path(s) => {
                    if !s.is_empty() {
                        out.push(s);
                    }
                }
                IncludePathItem::Generator(gen_fn) => {
                    let mut dynamic = gen_fn()?;
                    dynamic.extend(queue);
                    queue = dynamic;
                }
                IncludePathItem::Object(obj) => {
                    match obj.call_method("paths", &[], CallContext::List) {
                        MethodResult::Ok(Value::List(list_rc)) => {
                            let mut dynamic: Vec<IncludePathItem> = list_rc
                                .borrow()
                                .iter()
                                .map(|v| IncludePathItem::Path(v.to_tt_string()))
                                .collect();
                            dynamic.extend(queue);
                            queue = dynamic;
                        }
                        MethodResult::Err(cf) => {
                            return Err(format!("{:?}", cf));
                        }
                        _ => {
                            return Err("failed to get paths from object".to_string());
                        }
                    }
                }
            }
        }

        if !queue.is_empty() {
            let msg = format!("INCLUDE_PATH exceeds {} directories", max_dirs);
            *self.error_msg.borrow_mut() = msg.clone();
            return Err(msg);
        }

        Ok(out)
    }

    pub fn fetch_text(&self, text: &str, alias: Option<&str>) -> FetchResult {
        let name = alias.unwrap_or("input text").to_string();
        let now = current_unix_time();
        self.compile_source(text, &name, &name, now, None)
    }

    pub fn fetch(&self, name: &str) -> FetchResult {
        if Path::new(name).is_absolute() {
            if self.absolute {
                self.fetch_file(name, name)
            } else if self.tolerant {
                FetchResult::Declined
            } else {
                FetchResult::Error(TemplateException::new(
                    "file",
                    format!(
                        "{}: absolute paths are not allowed (set ABSOLUTE option)",
                        name
                    ),
                ))
            }
        } else if is_relative_path(name) {
            if self.relative {
                self.fetch_file(name, name)
            } else if self.tolerant {
                FetchResult::Declined
            } else {
                FetchResult::Error(TemplateException::new(
                    "file",
                    format!(
                        "{}: relative paths are not allowed (set RELATIVE option)",
                        name
                    ),
                ))
            }
        } else if !self.include_path.is_empty() {
            self.fetch_path(name)
        } else {
            FetchResult::Declined
        }
    }

    pub fn load(&self, name: &str) -> LoadResult {
        let mut target_path: Option<PathBuf> = None;

        if Path::new(name).is_absolute() {
            if !self.absolute {
                return if self.tolerant {
                    LoadResult::Declined
                } else {
                    LoadResult::Error(TemplateException::new(
                        "file",
                        format!(
                            "{}: absolute paths are not allowed (set ABSOLUTE option)",
                            name
                        ),
                    ))
                };
            }
            target_path = Some(PathBuf::from(name));
        } else if is_relative_path(name) {
            if !self.relative {
                return if self.tolerant {
                    LoadResult::Declined
                } else {
                    LoadResult::Error(TemplateException::new(
                        "file",
                        format!(
                            "{}: relative paths are not allowed (set RELATIVE option)",
                            name
                        ),
                    ))
                };
            }
            target_path = Some(PathBuf::from(name));
        } else {
            let paths = match self.paths() {
                Ok(p) => p,
                Err(e) => {
                    return LoadResult::Error(TemplateException::new("file", e));
                }
            };
            for dir in paths {
                let candidate = Path::new(&dir).join(name);
                if candidate.is_file() {
                    target_path = Some(candidate);
                    break;
                }
            }
        }

        let Some(path) = target_path else {
            return LoadResult::Declined;
        };

        if !path.is_file() {
            return LoadResult::Declined;
        }

        match fs::read(&path) {
            Ok(bytes) => {
                // Note: Provider::load() in TT2 reads raw bytes without BOM decoding,
                // or decodes according to ENCODING if set
                let content = self.decode_bytes(&bytes, false);
                LoadResult::Ok(content)
            }
            Err(e) => {
                if self.tolerant {
                    LoadResult::Declined
                } else {
                    LoadResult::Error(TemplateException::new(
                        "file",
                        format!("{}: {}", path.display(), e),
                    ))
                }
            }
        }
    }

    pub fn store(&self, name: &str, doc: Rc<Document>, mtime: Option<u64>) -> Rc<Document> {
        if self.cache_size == Some(0) {
            return doc;
        }

        let load_mtime = mtime.or_else(|| file_mtime(Path::new(name))).unwrap_or(0);
        let now = current_unix_time();
        let alias = doc.name();

        let mut order = self.cache_order.borrow_mut();
        let mut lookup = self.lookup.borrow_mut();

        if let Some(pos) = order.iter().position(|k| k == name) {
            order.remove(pos);
        } else if let Some(limit) = self.cache_size {
            if order.len() >= limit {
                if let Some(oldest) = order.pop() {
                    lookup.remove(&oldest);
                }
            }
        }

        order.insert(0, name.to_string());
        lookup.insert(
            name.to_string(),
            CacheSlot {
                name: name.to_string(),
                alias,
                doc: Rc::clone(&doc),
                load_mtime,
                stat_time: now,
            },
        );

        doc
    }

    fn fetch_path(&self, name: &str) -> FetchResult {
        if self.lookup.borrow().contains_key(name) {
            return self.refresh_slot(name);
        }

        let paths = match self.paths() {
            Ok(p) => p,
            Err(e) => return FetchResult::Error(TemplateException::new("file", e)),
        };

        for dir in paths {
            // File::Spec->catfile(".", $name) is "./$name"
            let full_path = format!("{}/{}", dir.trim_end_matches('/'), name);
            match self.fetch_file(&full_path, name) {
                FetchResult::Declined => continue,
                other => return other,
            }
        }

        if let Some(ref def_name) = self.default {
            if name != def_name {
                return self.fetch_path(def_name);
            }
        }

        FetchResult::Declined
    }

    fn fetch_file(&self, path: &str, alias: &str) -> FetchResult {
        if self.lookup.borrow().contains_key(path) {
            return self.refresh_slot(path);
        }

        let stat_ttl = self.stat_ttl.unwrap_or_else(|| get_stat_ttl() as u64);
        let now = current_unix_time();

        let last_nf = self.notfound.borrow().get(path).copied();
        if let Some(last_nf) = last_nf {
            if last_nf + stat_ttl > now {
                return FetchResult::Declined;
            } else {
                self.notfound.borrow_mut().remove(path);
            }
        }

        if Path::new(path).is_dir() {
            return if self.tolerant {
                FetchResult::Declined
            } else {
                FetchResult::Error(TemplateException::new(
                    "file",
                    format!("{}: not a file", path),
                ))
            };
        }

        let uncompiled_mtime = file_mtime(Path::new(path));
        let comp_file = self.compiled_filename(path);

        if let (Some(src_mtime), Some(ref cpath)) = (uncompiled_mtime, &comp_file) {
            if let Some(c_mtime) = file_mtime(cpath) {
                if c_mtime == src_mtime {
                    if let Ok(json_str) = fs::read_to_string(cpath) {
                        let mut extra = HashMap::new();
                        extra.insert("name".to_string(), Value::Str(alias.to_string()));
                        extra.insert("modtime".to_string(), Value::Int(c_mtime as i64));
                        if let Ok(doc) = Document::from_compiled_json(&json_str, Some(extra)) {
                            let doc_rc = Rc::new(doc);
                            return FetchResult::Ok(self.store(path, doc_rc, Some(c_mtime)));
                        }
                    }
                }
            }
        }

        let Some(src_mtime) = uncompiled_mtime else {
            self.notfound.borrow_mut().insert(path.to_string(), now);
            return FetchResult::Declined;
        };

        let bytes = match fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                self.notfound.borrow_mut().insert(path.to_string(), now);
                return if self.tolerant {
                    FetchResult::Declined
                } else {
                    FetchResult::Error(TemplateException::new("file", format!("{}: {}", path, e)))
                };
            }
        };

        let text = self.decode_bytes(&bytes, self.unicode);
        match self.compile_source(&text, alias, path, src_mtime, comp_file.as_deref()) {
            FetchResult::Ok(doc_rc) => FetchResult::Ok(self.store(path, doc_rc, Some(src_mtime))),
            other => other,
        }
    }

    fn refresh_slot(&self, key: &str) -> FetchResult {
        let stat_ttl = self.stat_ttl.unwrap_or_else(|| get_stat_ttl() as u64);
        let now = current_unix_time();

        let slot_opt = self.lookup.borrow().get(key).cloned();
        let Some(mut slot) = slot_opt else {
            return FetchResult::Declined;
        };

        // Touch LRU order
        {
            let mut order = self.cache_order.borrow_mut();
            if let Some(pos) = order.iter().position(|k| k == key) {
                order.remove(pos);
            }
            order.insert(0, key.to_string());
        }

        if slot.load_mtime > 0 && slot.stat_time + stat_ttl <= now {
            slot.stat_time = now;
            let current_mtime = file_mtime(Path::new(&slot.name));
            if current_mtime != Some(slot.load_mtime) {
                // Reload and recompile
                let Some(new_mtime) = current_mtime else {
                    slot.stat_time = 0;
                    self.lookup.borrow_mut().insert(key.to_string(), slot);
                    return FetchResult::Declined;
                };
                match fs::read(&slot.name) {
                    Ok(bytes) => {
                        let text = self.decode_bytes(&bytes, self.unicode);
                        let comp_file = self.compiled_filename(&slot.name);
                        match self.compile_source(
                            &text,
                            &slot.alias,
                            &slot.name,
                            new_mtime,
                            comp_file.as_deref(),
                        ) {
                            FetchResult::Ok(new_doc) => {
                                slot.doc = Rc::clone(&new_doc);
                                slot.load_mtime = new_mtime;
                                self.lookup.borrow_mut().insert(key.to_string(), slot);
                                return FetchResult::Ok(new_doc);
                            }
                            err => {
                                slot.stat_time = 0;
                                self.lookup.borrow_mut().insert(key.to_string(), slot);
                                return err;
                            }
                        }
                    }
                    Err(e) => {
                        slot.stat_time = 0;
                        self.lookup.borrow_mut().insert(key.to_string(), slot);
                        return FetchResult::Error(TemplateException::new("file", e.to_string()));
                    }
                }
            }
            self.lookup
                .borrow_mut()
                .insert(key.to_string(), slot.clone());
        }

        FetchResult::Ok(slot.doc)
    }

    fn compiled_filename(&self, file: &str) -> Option<PathBuf> {
        if self.compile_ext.is_none() && self.compile_dir.is_none() {
            return None;
        }
        let ext = self.compile_ext.as_deref().unwrap_or("");
        let with_ext = format!("{}{}", file, ext);
        if let Some(ref cdir) = self.compile_dir {
            let trimmed = with_ext.trim_start_matches('/');
            Some(Path::new(cdir.trim_end_matches('/')).join(trimmed))
        } else {
            Some(PathBuf::from(with_ext))
        }
    }

    fn compile_source(
        &self,
        text: &str,
        alias: &str,
        path: &str,
        mtime: u64,
        compfile: Option<&Path>,
    ) -> FetchResult {
        let info = ParseInfo {
            name: Some(alias.to_string()),
            path: Some(path.to_string()),
            debug: None,
        };
        let parsed_res = self.parser.borrow_mut().parse(text, Some(info));
        match parsed_res {
            Ok(parsed) => {
                let mut extra = HashMap::new();
                extra.insert("name".to_string(), Value::Str(alias.to_string()));
                extra.insert("modtime".to_string(), Value::Int(mtime as i64));
                let doc = Document::new(parsed, Some(extra));

                if let Some(cfile) = compfile {
                    if let Err(e) = doc.write_compiled_file(cfile) {
                        return if self.tolerant {
                            FetchResult::Declined
                        } else {
                            FetchResult::Error(TemplateException::new("file", e))
                        };
                    }
                    set_file_mtime(cfile, mtime);
                }

                FetchResult::Ok(Rc::new(doc))
            }
            Err(err_msg) => {
                if self.tolerant {
                    FetchResult::Declined
                } else {
                    FetchResult::Error(TemplateException::new(
                        "parse",
                        format!("{} {}", alias, err_msg),
                    ))
                }
            }
        }
    }

    pub fn decode_bytes(&self, bytes: &[u8], check_bom: bool) -> String {
        if check_bom {
            // Check BOMs in exact TT2 order:
            // UTF-8: EF BB BF
            // UTF-32BE: 00 00 FE FF
            // UTF-32LE: FF FE 00 00
            // UTF-16BE: FE FF
            // UTF-16LE: FF FE
            if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
                return String::from_utf8_lossy(&bytes[3..]).into_owned();
            }
            if bytes.starts_with(&[0x00, 0x00, 0xFE, 0xFF]) {
                let rest = &bytes[4..];
                let mut chars = Vec::with_capacity(rest.len() / 4);
                for chunk in rest.chunks_exact(4) {
                    let cp = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    if let Some(c) = char::from_u32(cp) {
                        chars.push(c);
                    }
                }
                return chars.into_iter().collect();
            }
            if bytes.starts_with(&[0xFF, 0xFE, 0x00, 0x00]) {
                let rest = &bytes[4..];
                let mut chars = Vec::with_capacity(rest.len() / 4);
                for chunk in rest.chunks_exact(4) {
                    let cp = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    if let Some(c) = char::from_u32(cp) {
                        chars.push(c);
                    }
                }
                return chars.into_iter().collect();
            }
            if bytes.starts_with(&[0xFE, 0xFF]) {
                let (cow, _, _) = encoding_rs::UTF_16BE.decode(&bytes[2..]);
                return cow.into_owned();
            }
            if bytes.starts_with(&[0xFF, 0xFE]) {
                let (cow, _, _) = encoding_rs::UTF_16LE.decode(&bytes[2..]);
                return cow.into_owned();
            }
        }

        if let Some(ref enc_name) = self.encoding {
            if let Some(enc) = encoding_rs::Encoding::for_label(enc_name.as_bytes()) {
                let (cow, _, _) = enc.decode(bytes);
                return cow.into_owned();
            }
        }

        match std::str::from_utf8(bytes) {
            Ok(s) => s.to_string(),
            Err(_) => {
                // Fallback: Latin-1 (byte-to-char 1:1, just like Perl non-utf8 byte strings)
                bytes.iter().map(|&b| b as char).collect()
            }
        }
    }
}
