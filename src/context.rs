use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    constants::{
        DEBUG_DIRS, DEBUG_UNDEF, ERROR_FILE, ERROR_PERL, ERROR_PLUGIN, ERROR_RETURN, ERROR_UNDEF,
        ERROR_VIEW, STATUS_DONE,
    },
    document::Document,
    error::{ControlFlow, TemplateException},
    filters::{FilterDef, FilterFn, Filters},
    iterator::TemplateIterator,
    parser::{
        ast::{AssignTarget, BinOp, Block, Expr, IdentNode, NamedArg, SpannedStmt, Stmt},
        Parser,
    },
    plugins::{load_builtin_plugin, PluginFactory},
    provider::{FetchResult, LoadResult, Provider},
    stash::Stash,
    value::{format_perl_float, CallContext, Callable, Value},
    view::View,
    vmethods::VMethod,
};

pub const DEFAULT_DEBUG_FORMAT: &str = "\n## $file line $line : [% $text %] ##\n";
pub const WHILE_MAX: usize = 1000;
static WHILE_MAX_CUR: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(WHILE_MAX);

/// Equivalent of `$Template::Directive::WHILE_MAX`.
pub fn set_while_max(n: usize) {
    WHILE_MAX_CUR.store(n, std::sync::atomic::Ordering::Relaxed);
}

pub fn get_while_max() -> usize {
    WHILE_MAX_CUR.load(std::sync::atomic::Ordering::Relaxed)
}

#[derive(Clone)]
pub enum CompiledTemplate {
    Document(Rc<Document>),
    Closure { name: String, func: BlockFn },
}

impl std::fmt::Debug for CompiledTemplate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompiledTemplate::Document(doc) => f.debug_tuple("Document").field(doc).finish(),
            CompiledTemplate::Closure { name, .. } => {
                f.debug_struct("Closure").field("name", name).finish()
            }
        }
    }
}

/// A native block body: renders output given the current context.
pub type BlockFn = Rc<dyn Fn(&Context) -> Result<String, ControlFlow>>;

#[derive(Clone)]
pub enum BlockDef {
    Text(String),
    Document(Rc<Document>),
    Closure(BlockFn),
}

impl From<&str> for BlockDef {
    fn from(s: &str) -> Self {
        BlockDef::Text(s.to_string())
    }
}

impl From<String> for BlockDef {
    fn from(s: String) -> Self {
        BlockDef::Text(s)
    }
}

impl From<Rc<Document>> for BlockDef {
    fn from(d: Rc<Document>) -> Self {
        BlockDef::Document(d)
    }
}

impl From<Document> for BlockDef {
    fn from(d: Document) -> Self {
        BlockDef::Document(Rc::new(d))
    }
}

#[derive(Clone)]
pub struct Context {
    inner: Rc<RefCell<ContextInner>>,
}

impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context").finish()
    }
}

struct ContextInner {
    load_templates: Vec<Rc<RefCell<Provider>>>,
    prefix_map: HashMap<String, Vec<Rc<RefCell<Provider>>>>,
    load_filters: Vec<Rc<RefCell<Filters>>>,
    filter_cache: HashMap<String, FilterFn>,
    plugin_factories: HashMap<String, Rc<dyn PluginFactory>>,
    plugin_base: Vec<String>,
    stash: Stash,
    init_blocks: HashMap<String, CompiledTemplate>,
    blocks: HashMap<String, CompiledTemplate>,
    blkstack: Vec<HashMap<String, Rc<Document>>>,
    recursion: bool,
    eval_perl: bool,
    trim: bool,
    expose_blocks: bool,
    debug_format: Option<String>,
    debug_dirs: bool,
    current_line: String,
    current_file: String,
    current_from_compiled: bool,
}

#[bon::bon]
impl Context {
    #[builder(start_fn = builder, finish_fn = build)]
    pub fn build(
        load_templates: Option<Vec<Rc<RefCell<Provider>>>>,
        prefix_map: Option<HashMap<String, Vec<Rc<RefCell<Provider>>>>>,
        load_filters: Option<Vec<Rc<RefCell<Filters>>>>,
        plugin_factories: Option<HashMap<String, Rc<dyn PluginFactory>>>,
        plugin_base: Option<Vec<String>>,
        stash: Option<Stash>,
        variables: Option<HashMap<String, Value>>,
        blocks: Option<HashMap<String, BlockDef>>,
        views: Option<Vec<(String, HashMap<String, Value>)>>,
        #[builder(default = false)] recursion: bool,
        #[builder(default = false)] eval_perl: bool,
        #[builder(default = false)] trim: bool,
        #[builder(default = false)] expose_blocks: bool,
        debug_format: Option<String>,
        #[builder(default = 0)] debug: u32,
        #[builder(default = false)] strict: bool,
    ) -> Result<Self, TemplateException> {
        let providers = match load_templates {
            Some(p) => p,
            None => vec![Rc::new(RefCell::new(
                Provider::builder()
                    .build()
                    .map_err(|e| TemplateException::new("file", e))?,
            ))],
        };
        let filters = load_filters.unwrap_or_else(|| vec![Rc::new(RefCell::new(Filters::new()))]);
        let stash_obj = match stash {
            Some(mut s) => {
                if let Some(vars) = variables {
                    s.update(vars);
                }
                if strict {
                    s.strict = true;
                }
                if (debug & DEBUG_UNDEF) != 0 {
                    s.debug = true;
                }
                s
            }
            None => Stash::builder()
                .maybe_params(variables)
                .strict(strict)
                .debug((debug & DEBUG_UNDEF) != 0)
                .build(),
        };

        let ctx = Self {
            inner: Rc::new(RefCell::new(ContextInner {
                load_templates: providers,
                prefix_map: prefix_map.unwrap_or_default(),
                load_filters: filters,
                filter_cache: HashMap::new(),
                plugin_factories: plugin_factories.unwrap_or_default(),
                plugin_base: plugin_base.unwrap_or_default(),
                stash: stash_obj,
                init_blocks: HashMap::new(),
                blocks: HashMap::new(),
                blkstack: Vec::new(),
                recursion,
                eval_perl,
                trim,
                expose_blocks,
                debug_format,
                debug_dirs: (debug & DEBUG_DIRS) != 0,
                current_line: "1".to_string(),
                current_file: "input text".to_string(),
                current_from_compiled: false,
            })),
        };

        if let Some(blk_defs) = blocks {
            let mut compiled_map = HashMap::new();
            for (name, def) in blk_defs {
                let comp = match def {
                    BlockDef::Document(d) => CompiledTemplate::Document(d),
                    BlockDef::Closure(f) => CompiledTemplate::Closure {
                        name: name.clone(),
                        func: f,
                    },
                    BlockDef::Text(txt) => {
                        let doc = ctx.compile_text(&txt)?;
                        CompiledTemplate::Document(doc)
                    }
                };
                compiled_map.insert(name, comp);
            }
            let mut inner = ctx.inner.borrow_mut();
            inner.init_blocks = compiled_map.clone();
            inner.blocks = compiled_map;
        }

        if let Some(view_defs) = views {
            ctx.define_views(view_defs)?;
        }

        Ok(ctx)
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::builder().build().expect("default context")
    }
}

impl Context {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stash(&self) -> Stash {
        self.inner.borrow().stash.clone()
    }

    pub fn set_stash(&self, stash: Stash) {
        self.inner.borrow_mut().stash = stash;
    }

    pub fn trim(&self) -> bool {
        self.inner.borrow().trim
    }

    pub fn eval_perl(&self) -> bool {
        self.inner.borrow().eval_perl
    }

    pub fn recursion(&self) -> bool {
        self.inner.borrow().recursion
    }

    pub fn expose_blocks(&self) -> bool {
        self.inner.borrow().expose_blocks
    }

    pub fn providers(&self) -> Vec<Rc<RefCell<Provider>>> {
        self.inner.borrow().load_templates.clone()
    }

    pub fn register_plugin(&self, name: impl Into<String>, factory: Rc<dyn PluginFactory>) {
        self.inner
            .borrow_mut()
            .plugin_factories
            .insert(name.into(), factory);
    }

    pub fn compile_text(&self, text: &str) -> Result<Rc<Document>, TemplateException> {
        let providers = self.inner.borrow().load_templates.clone();
        for prov in providers {
            match prov.borrow().fetch_text(text, None) {
                FetchResult::Ok(doc) => return Ok(doc),
                FetchResult::Declined => continue,
                FetchResult::Error(e) => {
                    if e.type_ == ERROR_FILE {
                        return Err(e);
                    }
                    return Err(TemplateException::new(ERROR_FILE, e.to_string()));
                }
            }
        }
        let mut parser = Parser::new();
        match parser.parse(text, None) {
            Ok(parsed) => Ok(Rc::new(Document::new(parsed, None))),
            Err(e) => Err(TemplateException::new("parse", format!("input text {}", e))),
        }
    }

    pub fn template(&self, name: &str) -> Result<Rc<Document>, TemplateException> {
        match self.template_by_name(name)? {
            CompiledTemplate::Document(d) => Ok(d),
            CompiledTemplate::Closure { name, func } => {
                // If caller needs a Document, evaluate or wrap?
                // Wait: in View::new or View::template, blocks are Documents.
                let _ = func;
                Ok(Rc::new(Document::from_block(Block::default(), Some(name))))
            }
        }
    }

    pub fn template_by_name(&self, name: &str) -> Result<CompiledTemplate, TemplateException> {
        // 1. Check BLOCKS
        if let Some(found) = self.inner.borrow().blocks.get(name).cloned() {
            return Ok(found);
        }

        // 2. Check BLKSTACK
        {
            let inner = self.inner.borrow();
            for blk_map in &inner.blkstack {
                if let Some(doc) = blk_map.get(name) {
                    return Ok(CompiledTemplate::Document(Rc::clone(doc)));
                }
            }
        }

        // 3. Check prefix and ask providers
        let mut shortname = name.to_string();
        let mut prefix: Option<String> = None;
        if let Some(colon_pos) = shortname.find(':') {
            let candidate = &shortname[..colon_pos];
            if !candidate.is_empty()
                && candidate
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                prefix = Some(candidate.to_string());
                shortname = shortname[colon_pos + 1..].to_string();
            }
        }

        let providers = {
            let inner = self.inner.borrow();
            if let Some(ref pfx) = prefix {
                match inner.prefix_map.get(pfx) {
                    Some(p) => p.clone(),
                    None => {
                        return Err(TemplateException::new(
                            ERROR_FILE,
                            format!("no providers for template prefix '{}'", pfx),
                        ));
                    }
                }
            } else if let Some(def_p) = inner.prefix_map.get("default") {
                def_p.clone()
            } else {
                inner.load_templates.clone()
            }
        };

        let expose_blocks = self.inner.borrow().expose_blocks;
        let mut blockname = String::new();

        while !shortname.is_empty() {
            for provider in &providers {
                let res = provider.borrow().fetch(&shortname);
                match res {
                    FetchResult::Ok(doc) => {
                        if !blockname.is_empty() {
                            if let Some(blk) = doc.defblocks.get(&blockname) {
                                let mut blk_doc =
                                    Document::from_block(blk.clone(), Some(blockname.clone()));
                                blk_doc.from_compiled = doc.from_compiled;
                                return Ok(CompiledTemplate::Document(Rc::new(blk_doc)));
                            }
                        } else {
                            return Ok(CompiledTemplate::Document(doc));
                        }
                    }
                    FetchResult::Declined => continue,
                    FetchResult::Error(err) => {
                        if err.type_ == ERROR_FILE {
                            return Err(err);
                        }
                        let msg = err.to_string();
                        return Err(TemplateException::new(ERROR_FILE, msg));
                    }
                }
            }

            if !expose_blocks {
                break;
            }
            if let Some(slash_pos) = shortname.rfind('/') {
                let tail = &shortname[slash_pos + 1..];
                if tail.is_empty() {
                    break;
                }
                blockname = if blockname.is_empty() {
                    tail.to_string()
                } else {
                    format!("{}/{}", tail, blockname)
                };
                shortname.truncate(slash_pos);
            } else {
                break;
            }
        }

        Err(TemplateException::new(
            ERROR_FILE,
            format!("{}: not found", name),
        ))
    }

    pub fn template_item(&self, item: &Value) -> Result<CompiledTemplate, TemplateException> {
        match item {
            Value::Object(obj) => {
                if let Some(doc) = obj.as_any().downcast_ref::<Document>() {
                    return Ok(CompiledTemplate::Document(Rc::new(doc.clone())));
                }
                self.template_by_name(&item.to_tt_string())
            }
            Value::Function(callable) => {
                let c = Rc::clone(callable);
                Ok(CompiledTemplate::Closure {
                    name: String::new(),
                    func: Rc::new(move |ctx: &Context| {
                        let res = c.call(&[Value::Undefined], CallContext::Scalar)?;
                        let _ = ctx;
                        Ok(res.to_tt_string())
                    }),
                })
            }
            Value::ScalarRef(s_rc) => {
                let text = s_rc.borrow().clone();
                let doc = self.compile_text(&text)?;
                Ok(CompiledTemplate::Document(doc))
            }
            _ => self.template_by_name(&item.to_tt_string()),
        }
    }

    pub fn plugin(&self, name: &str, args: &[Value]) -> Result<Value, TemplateException> {
        // 1. Check custom registered plugin factories first
        let custom_factory = {
            let inner = self.inner.borrow();
            inner.plugin_factories.get(name).cloned().or_else(|| {
                inner
                    .plugin_factories
                    .get(&name.to_ascii_lowercase())
                    .cloned()
            })
        };
        if let Some(factory) = custom_factory {
            return factory.load(self, args);
        }

        // 2. Builtin plugins
        let plugin_base = self.inner.borrow().plugin_base.clone();
        if let Some(val) = load_builtin_plugin(name, args, self, &plugin_base)? {
            return Ok(val);
        }

        Err(TemplateException::new(
            ERROR_PLUGIN,
            format!("{}: plugin not found", name),
        ))
    }

    pub fn get_filter(&self, name: &str, args: &[Value]) -> Result<FilterFn, ControlFlow> {
        self.filter(name, if args.is_empty() { None } else { Some(args) }, None)
    }

    pub fn filter(
        &self,
        name: &str,
        args: Option<&[Value]>,
        alias: Option<&str>,
    ) -> Result<FilterFn, ControlFlow> {
        if args.is_none() {
            if let Some(cached) = self.inner.borrow().filter_cache.get(name).cloned() {
                return Ok(cached);
            }
        }

        // Special handling for `eval` / `evaltt` filter which needs Context
        if (name == "eval" || name == "evaltt") && args.is_none() {
            let ctx = self.clone();
            let f: FilterFn = Rc::new(move |text: &str| {
                let doc = ctx.compile_text(text).map_err(ControlFlow::Exception)?;
                let compiled = CompiledTemplate::Document(doc);
                ctx.process_compiled_list(&[compiled], None, false)
            });
            if let Some(al) = alias {
                if !al.is_empty() {
                    self.inner
                        .borrow_mut()
                        .filter_cache
                        .insert(al.to_string(), Rc::clone(&f));
                }
            }
            return Ok(f);
        }

        let providers = self.inner.borrow().load_filters.clone();
        let arg_slice = args.unwrap_or(&[]);
        let mut found_filter: Option<FilterFn> = None;

        for prov in providers {
            let res = prov.borrow().fetch(name, arg_slice, self);
            match res {
                Ok(Some(f)) => {
                    found_filter = Some(f);
                    break;
                }
                Ok(None) => continue,
                Err(e) => return Err(ControlFlow::Exception(e)),
            }
        }

        // Also check if `name` is a Value::Function in stash or filter_cache?
        // Perl sets a plain-string error here, which the FILTER directive
        // rethrows via `$context->throw`, so the exception type is 'undef'.
        let Some(f) = found_filter else {
            return Err(ControlFlow::Exception(TemplateException::new(
                ERROR_UNDEF,
                format!("{}: filter not found", name),
            )));
        };

        if let Some(al) = alias {
            if !al.is_empty() {
                self.inner
                    .borrow_mut()
                    .filter_cache
                    .insert(al.to_string(), Rc::clone(&f));
            }
        }

        Ok(f)
    }

    pub fn define_filter(&self, name: impl Into<String>, def: FilterDef) {
        let name_str = name.into();
        let providers = self.inner.borrow().load_filters.clone();
        if let Some(first) = providers.first() {
            first.borrow_mut().store(name_str, def);
        }
    }

    pub fn define_block(&self, name: impl Into<String>, block: CompiledTemplate) {
        self.inner.borrow_mut().blocks.insert(name.into(), block);
    }

    pub fn define_block_text(
        &self,
        name: impl Into<String>,
        text: &str,
    ) -> Result<(), TemplateException> {
        let doc = self.compile_text(text)?;
        self.define_block(name, CompiledTemplate::Document(doc));
        Ok(())
    }

    pub fn define_vmethod(&self, vtype: &str, name: &str, f: impl VMethod + 'static) {
        let inner = self.inner.borrow();
        if let Some(vm) = &inner.stash.vmethods {
            vm.borrow_mut().define_vmethod(vtype, name, f);
        } else {
            // no per-stash registry: fall back to the global ops tables
            // (cf. $Template::Stash::LIST_OPS etc.)
            crate::vmethods::define_global_vmethod(vtype, name, f);
        }
    }

    pub fn view(&self, params: HashMap<String, Value>) -> Result<View, TemplateException> {
        View::new(self.clone(), params)
    }

    pub fn define_view(
        &self,
        name: &str,
        mut params: HashMap<String, Value>,
    ) -> Result<View, TemplateException> {
        if let Some(base_val) = params.get("base").cloned() {
            if matches!(base_val, Value::Str(_)) {
                let resolved = self
                    .stash()
                    .get(&base_val, None)
                    .unwrap_or(Value::Undefined);
                if !resolved.is_truthy() {
                    return Err(TemplateException::new(
                        ERROR_VIEW,
                        format!("view base is not defined: {}", base_val.to_tt_string()),
                    ));
                }
                let is_view = match &resolved {
                    Value::Object(o) => o.as_any().downcast_ref::<View>().is_some(),
                    _ => false,
                };
                if !is_view {
                    return Err(TemplateException::new(
                        ERROR_VIEW,
                        format!(
                            "view base is not a Template::View object: {} => {}",
                            base_val.to_tt_string(),
                            resolved.to_tt_string()
                        ),
                    ));
                }
                params.insert("base".to_string(), resolved);
            }
        }
        let view = self.view(params)?;
        view.seal();
        let _ = self.stash().set(
            &Value::Str(name.to_string()),
            Value::Object(Rc::new(view.clone())),
            false,
        );
        Ok(view)
    }

    pub fn define_views(
        &self,
        views: Vec<(String, HashMap<String, Value>)>,
    ) -> Result<(), TemplateException> {
        for (name, params) in views {
            self.define_view(&name, params)?;
        }
        Ok(())
    }

    pub fn localise(&self, params: Option<HashMap<String, Value>>) -> Stash {
        let mut inner = self.inner.borrow_mut();
        let cloned = inner.stash.clone_stash(params);
        inner.stash = cloned.clone();
        cloned
    }

    pub fn delocalise(&self) -> Stash {
        let mut inner = self.inner.borrow_mut();
        let current = inner.stash.clone();
        let parent = current.declone();
        inner.stash = parent.clone();
        parent
    }

    pub fn visit(&self, blocks: HashMap<String, Rc<Document>>) {
        self.inner.borrow_mut().blkstack.insert(0, blocks);
    }

    pub fn leave(&self) {
        let mut inner = self.inner.borrow_mut();
        if !inner.blkstack.is_empty() {
            inner.blkstack.remove(0);
        }
    }

    pub fn reset(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.blkstack.clear();
        inner.blocks = inner.init_blocks.clone();
    }

    pub fn debugging(&self, args: &[Value], params: Option<&HashMap<String, Value>>) -> String {
        let mut slice = args;
        if let Some(first) = slice.first() {
            let s = first.to_tt_string();
            let lower = s.to_ascii_lowercase();
            if s == "1" || lower == "on" {
                self.inner.borrow_mut().debug_dirs = true;
                slice = &slice[1..];
            } else if s == "0" || lower == "off" {
                self.inner.borrow_mut().debug_dirs = false;
                slice = &slice[1..];
            }
        }

        if let Some(first) = slice.first() {
            let lower = first.to_tt_string().to_ascii_lowercase();
            if lower == "msg" {
                let (debug_dirs, fmt) = {
                    let inner = self.inner.borrow();
                    (
                        inner.debug_dirs,
                        inner
                            .debug_format
                            .clone()
                            .unwrap_or_else(|| DEFAULT_DEBUG_FORMAT.to_string()),
                    )
                };
                if !debug_dirs {
                    return String::new();
                }
                let empty = HashMap::new();
                let map = params.unwrap_or(&empty);
                let mut out = String::with_capacity(fmt.len() + 32);
                let chars: Vec<char> = fmt.chars().collect();
                let mut i = 0;
                while i < chars.len() {
                    if chars[i] == '$'
                        && i + 1 < chars.len()
                        && (chars[i + 1].is_ascii_alphanumeric() || chars[i + 1] == '_')
                    {
                        i += 1;
                        let start = i;
                        while i < chars.len()
                            && (chars[i].is_ascii_alphanumeric() || chars[i] == '_')
                        {
                            i += 1;
                        }
                        let key: String = chars[start..i].iter().collect();
                        if let Some(val) = map.get(&key) {
                            out.push_str(&val.to_tt_string());
                        }
                    } else {
                        out.push(chars[i]);
                        i += 1;
                    }
                }
                return out;
            } else if lower == "format" {
                if let Some(fmt_val) = slice.get(1) {
                    self.inner.borrow_mut().debug_format = Some(fmt_val.to_tt_string());
                }
            }
        }

        String::new()
    }

    pub fn process(
        &self,
        template: &str,
        params: Option<HashMap<String, Value>>,
    ) -> Result<String, ControlFlow> {
        self.process_value(&Value::Str(template.to_string()), params, false)
    }

    pub fn include(
        &self,
        template: &str,
        params: Option<HashMap<String, Value>>,
    ) -> Result<String, ControlFlow> {
        self.process_value(&Value::Str(template.to_string()), params, true)
    }

    pub fn include_value(
        &self,
        template: &Value,
        params: Option<HashMap<String, Value>>,
    ) -> Result<String, ControlFlow> {
        self.process_value(template, params, true)
    }

    pub fn process_value(
        &self,
        template: &Value,
        params: Option<HashMap<String, Value>>,
        localize: bool,
    ) -> Result<String, ControlFlow> {
        let items: Vec<Value> = match template {
            Value::List(l_rc) => l_rc.borrow().clone(),
            other => vec![other.clone()],
        };

        let mut compiled_list = Vec::with_capacity(items.len());
        for item in &items {
            let comp = self.template_item(item).map_err(ControlFlow::Exception)?;
            compiled_list.push(comp);
        }

        self.process_compiled_list(&compiled_list, params, localize)
    }

    pub fn process_compiled_list(
        &self,
        compiled_list: &[CompiledTemplate],
        params: Option<HashMap<String, Value>>,
        localize: bool,
    ) -> Result<String, ControlFlow> {
        let stash = if localize {
            self.localise(params)
        } else {
            let mut s = self.stash();
            if let Some(p) = params {
                s.update(p);
            }
            s
        };

        let run_result: Result<String, ControlFlow> = (|| {
            let mut output = String::new();
            let prev_component = stash
                .get(&Value::Str("component".to_string()), None)
                .unwrap_or(Value::Undefined);

            for compiled in compiled_list {
                let element_val = match compiled {
                    CompiledTemplate::Document(doc) => {
                        if let Value::Object(ref prev_obj) = prev_component {
                            if let Some(prev_doc) = prev_obj.as_any().downcast_ref::<Document>() {
                                let caller_name = prev_doc.name();
                                doc.set_meta("caller", Value::Str(caller_name.clone()));
                                let mut callers_vec = match prev_doc.get_meta("callers") {
                                    Some(Value::List(l)) => l.borrow().clone(),
                                    _ => Vec::new(),
                                };
                                callers_vec.push(Value::Str(caller_name));
                                doc.set_meta("callers", Value::from_list(callers_vec));
                            }
                        }
                        Value::Object(Rc::clone(doc) as Rc<dyn crate::value::TemplateObject>)
                    }
                    CompiledTemplate::Closure { name, .. } => {
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(0);
                        let mut m = HashMap::new();
                        m.insert("name".to_string(), Value::Str(name.clone()));
                        m.insert("modtime".to_string(), Value::Int(now));
                        if let Value::Object(ref prev_obj) = prev_component {
                            if let Some(prev_doc) = prev_obj.as_any().downcast_ref::<Document>() {
                                let caller_name = prev_doc.name();
                                m.insert("caller".to_string(), Value::Str(caller_name.clone()));
                                let mut callers_vec = match prev_doc.get_meta("callers") {
                                    Some(Value::List(l)) => l.borrow().clone(),
                                    _ => Vec::new(),
                                };
                                callers_vec.push(Value::Str(caller_name));
                                m.insert("callers".to_string(), Value::from_list(callers_vec));
                            }
                        }
                        Value::from_hash(m)
                    }
                };

                let _ = stash.set(&Value::Str("component".to_string()), element_val, false);

                if !localize {
                    if let CompiledTemplate::Document(doc) = compiled {
                        let mut inner = self.inner.borrow_mut();
                        for (bname, bblock) in &doc.defblocks {
                            let mut bdoc =
                                Document::from_block(bblock.clone(), Some(bname.clone()));
                            bdoc.from_compiled = doc.from_compiled;
                            inner
                                .blocks
                                .insert(bname.clone(), CompiledTemplate::Document(Rc::new(bdoc)));
                        }
                    }
                }

                let mut tmpout = match compiled {
                    CompiledTemplate::Document(doc) => self.eval_document(doc)?,
                    CompiledTemplate::Closure { func, .. } => func(self)?,
                };

                if self.trim() {
                    tmpout = tmpout.trim().to_string();
                }
                output.push_str(&tmpout);

                if let CompiledTemplate::Document(doc) = compiled {
                    if let Value::Object(ref prev_obj) = prev_component {
                        if prev_obj.as_any().downcast_ref::<Document>().is_some() {
                            if let Some(Value::List(callers_rc)) = doc.get_meta("callers") {
                                callers_rc.borrow_mut().pop();
                            }
                        }
                    }
                }
            }

            let _ = stash.set(&Value::Str("component".to_string()), prev_component, false);

            Ok(output)
        })();

        if localize {
            self.delocalise();
        }

        run_result
    }

    pub fn insert(&self, file: &str) -> Result<String, ControlFlow> {
        self.insert_value(&Value::Str(file.to_string()))
    }

    pub fn insert_value(&self, file: &Value) -> Result<String, ControlFlow> {
        let files: Vec<Value> = match file {
            Value::List(l_rc) => l_rc.borrow().clone(),
            other => vec![other.clone()],
        };

        let mut output = String::new();
        for fval in files {
            let orig_name = fval.to_tt_string();
            let mut name = orig_name.clone();
            let mut prefix: Option<String> = None;
            if let Some(colon_pos) = name.find(':') {
                let candidate = &name[..colon_pos];
                if !candidate.is_empty()
                    && candidate
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_')
                {
                    prefix = Some(candidate.to_string());
                    name = name[colon_pos + 1..].to_string();
                }
            }

            let providers = {
                let inner = self.inner.borrow();
                if let Some(ref pfx) = prefix {
                    match inner.prefix_map.get(pfx) {
                        Some(p) => p.clone(),
                        None => {
                            return Err(ControlFlow::Exception(TemplateException::new(
                                ERROR_FILE,
                                format!("no providers for file prefix '{}'", pfx),
                            )));
                        }
                    }
                } else if let Some(def_p) = inner.prefix_map.get("default") {
                    def_p.clone()
                } else {
                    inner.load_templates.clone()
                }
            };

            let mut loaded_text: Option<String> = None;
            for prov in providers {
                match prov.borrow().load(&name) {
                    LoadResult::Ok(txt) => {
                        loaded_text = Some(txt);
                        break;
                    }
                    LoadResult::Declined => continue,
                    LoadResult::Error(e) => return Err(ControlFlow::Exception(e)),
                }
            }

            match loaded_text {
                Some(txt) => output.push_str(&txt),
                None => {
                    return Err(ControlFlow::Exception(TemplateException::new(
                        ERROR_FILE,
                        format!("{}: not found", orig_name),
                    )));
                }
            }
        }

        Ok(output)
    }

    pub fn eval_document(&self, doc: &Rc<Document>) -> Result<String, ControlFlow> {
        if doc.hot.get() && !doc.is_block && !self.recursion() {
            return Err(ControlFlow::Exception(TemplateException::new(
                ERROR_FILE,
                format!("recursion into '{}'", doc.name()),
            )));
        }

        let mut defblock_docs = HashMap::new();
        for (k, blk) in &doc.defblocks {
            let mut bdoc = Document::from_block(blk.clone(), Some(k.clone()));
            bdoc.from_compiled = doc.from_compiled;
            defblock_docs.insert(k.clone(), Rc::new(bdoc));
        }

        self.visit(defblock_docs);
        doc.hot.set(true);

        let prev_file = self.inner.borrow().current_file.clone();
        let prev_line = self.inner.borrow().current_line.clone();
        let prev_compiled = self.inner.borrow().current_from_compiled;
        {
            let mut inner = self.inner.borrow_mut();
            let dname = doc.name();
            if !dname.is_empty() {
                inner.current_file = dname;
            }
            inner.current_from_compiled = doc.from_compiled;
        }

        let res = self.eval_template_block(&doc.block);

        {
            let mut inner = self.inner.borrow_mut();
            inner.current_file = prev_file;
            inner.current_line = prev_line;
            inner.current_from_compiled = prev_compiled;
        }
        doc.hot.set(false);
        self.leave();

        res
    }

    pub fn eval_template_block(&self, block: &Block) -> Result<String, ControlFlow> {
        let mut output = String::new();
        match self.eval_block(block, &mut output) {
            Ok(()) => Ok(output),
            Err(cf) => {
                let ex = self.catch_control_flow(cf, &mut output)?;
                if ex.type_() == ERROR_RETURN {
                    Ok(output)
                } else {
                    Err(ControlFlow::Exception(ex))
                }
            }
        }
    }

    fn catch_control_flow(
        &self,
        cf: ControlFlow,
        output: &mut String,
    ) -> Result<TemplateException, ControlFlow> {
        match cf {
            ControlFlow::Exception(mut ex) => {
                if !ex.text().is_empty() {
                    output.push_str(ex.text());
                }
                ex.text = output.clone();
                Ok(ex)
            }
            ControlFlow::Return(txt) => {
                if !txt.is_empty() {
                    output.push_str(&txt);
                }
                Ok(TemplateException::with_text(
                    ERROR_RETURN,
                    "",
                    output.clone(),
                ))
            }
            ControlFlow::Stop(txt) => {
                if !txt.is_empty() {
                    output.push_str(&txt);
                }
                Ok(TemplateException::with_text("stop", "", output.clone()))
            }
            other => Err(other),
        }
    }

    pub fn eval_block(&self, block: &Block, output: &mut String) -> Result<(), ControlFlow> {
        for spanned in &block.stmts {
            self.eval_spanned_stmt(spanned, output)?;
        }
        Ok(())
    }

    fn eval_spanned_stmt(
        &self,
        spanned: &SpannedStmt,
        output: &mut String,
    ) -> Result<(), ControlFlow> {
        if spanned.line.is_some() || spanned.file.is_some() {
            let mut inner = self.inner.borrow_mut();
            if let Some(ref l) = spanned.line {
                inner.current_line = l.clone();
            }
            if let Some(ref f) = spanned.file {
                inner.current_file = f.clone();
            }
        }
        self.eval_stmt(&spanned.stmt, output)
    }

    fn eval_stmt(&self, stmt: &Stmt, output: &mut String) -> Result<(), ControlFlow> {
        match stmt {
            Stmt::Text(s) => {
                output.push_str(s);
                Ok(())
            }
            Stmt::Get(expr) => {
                let val = self.eval_expr(expr)?;
                output.push_str(&val.to_tt_string());
                Ok(())
            }
            Stmt::Call(expr) => {
                let _ = self.eval_expr(expr)?;
                Ok(())
            }
            Stmt::Set {
                assigns,
                is_default,
            } => {
                for (target, val_expr) in assigns {
                    let val = self.eval_expr(val_expr)?;
                    self.eval_assign(target, val, *is_default)?;
                }
                Ok(())
            }
            Stmt::Insert { files, args: _ } => {
                let file_val = self.eval_filenames(files)?;
                let inserted = self.insert_value(&file_val)?;
                output.push_str(&inserted);
                Ok(())
            }
            Stmt::Include { files, args } => {
                let file_val = self.eval_filenames(files)?;
                let params = self.eval_named_args(&args.named)?;
                let included = self.include_value(&file_val, params)?;
                output.push_str(&included);
                Ok(())
            }
            Stmt::Process { files, args } => {
                let file_val = self.eval_filenames(files)?;
                let params = self.eval_named_args(&args.named)?;
                let processed = self.process_value(&file_val, params, false)?;
                output.push_str(&processed);
                Ok(())
            }
            Stmt::Throw { types, args } => {
                let type_val = if let Some(first) = types.first() {
                    self.eval_expr(first)?
                } else {
                    Value::Undefined
                };

                // Rethrow an exception held in an object wrapper (e.g. `THROW error`)
                if let Value::Object(o) = &type_val {
                    if let Some(ex) = o.as_any().downcast_ref::<TemplateException>() {
                        return Err(ControlFlow::Exception(ex.clone()));
                    }
                }
                // If throwing an existing TemplateException object directly
                if let Value::Exception(ex_rc) = &type_val {
                    let mut ex = ex_rc.borrow().clone();
                    ex.text = String::new();
                    return Err(ControlFlow::Exception(ex));
                }

                let mut pos_vals = Vec::with_capacity(args.positional.len());
                for p in &args.positional {
                    pos_vals.push(self.eval_expr(p)?);
                }
                let named_map = self.eval_named_args(&args.named)?;

                if pos_vals.is_empty() && named_map.is_none() {
                    // `$context->throw($type, undef, \$output)` -> type 'undef', info = $type
                    let ex = TemplateException::new(ERROR_UNDEF, type_val.to_tt_string());
                    return Err(ControlFlow::Exception(ex));
                }

                let err_type = type_val.to_tt_string();
                let info_val = if pos_vals.len() == 1 && named_map.is_none() {
                    pos_vals.into_iter().next().unwrap()
                } else {
                    let mut hash = named_map.unwrap_or_default();
                    for (idx, pv) in pos_vals.iter().enumerate() {
                        hash.insert(idx.to_string(), pv.clone());
                    }
                    hash.insert("args".to_string(), Value::from_list(pos_vals));
                    Value::from_hash(hash)
                };

                Err(ControlFlow::Exception(TemplateException::new(
                    err_type, info_val,
                )))
            }
            Stmt::Return => Err(ControlFlow::Exception(TemplateException::new(
                ERROR_RETURN,
                "",
            ))),
            Stmt::Stop => Err(ControlFlow::Exception(TemplateException::new("stop", ""))),
            Stmt::Clear => {
                output.clear();
                Ok(())
            }
            Stmt::Last { label } => Err(ControlFlow::Last(label.clone())),
            Stmt::Next { label, .. } => Err(ControlFlow::Next(label.clone())),
            Stmt::Debug { names, args } => {
                let mut all_args = Vec::with_capacity(names.len() + args.positional.len());
                for n in names {
                    all_args.push(self.eval_expr(n)?);
                }
                for p in &args.positional {
                    all_args.push(self.eval_expr(p)?);
                }
                let named = self.eval_named_args(&args.named)?;
                let msg = self.debugging(&all_args, named.as_ref());
                output.push_str(&msg);
                Ok(())
            }
            Stmt::Wrapper { files, args, block } => {
                let mut sub_output = String::new();
                self.eval_block(block, &mut sub_output)?;

                let mut file_items = Vec::new();
                for f_expr in files {
                    let v = self.eval_expr(f_expr)?;
                    if let Value::List(l_rc) = v {
                        file_items.extend(l_rc.borrow().iter().cloned());
                    } else {
                        file_items.push(v);
                    }
                }
                let base_params = self.eval_named_args(&args.named)?.unwrap_or_default();

                for f_item in file_items.into_iter().rev() {
                    let mut p = base_params.clone();
                    p.insert("content".to_string(), Value::Str(sub_output));
                    sub_output = self.include_value(&f_item, Some(p))?;
                }
                output.push_str(&sub_output);
                Ok(())
            }
            Stmt::Filter {
                names,
                args,
                alias,
                block,
            } => {
                let name_val = if let Some(first) = names.first() {
                    self.eval_expr(first)?
                } else {
                    Value::Str(String::new())
                };
                let filter_name = name_val.to_tt_string();
                let mut pos_vals = Vec::with_capacity(args.positional.len() + 1);
                for p in &args.positional {
                    pos_vals.push(self.eval_expr(p)?);
                }
                if let Some(named_map) = self.eval_named_args(&args.named)? {
                    pos_vals.push(Value::from_hash(named_map));
                }
                let alias_str = match alias {
                    Some(AssignTarget::Ident(nodes)) => {
                        if let Some(first) = nodes.first() {
                            Some(self.eval_expr(&first.key)?.to_tt_string())
                        } else {
                            None
                        }
                    }
                    Some(AssignTarget::Expr(e)) => Some(self.eval_expr(e)?.to_tt_string()),
                    None => None,
                };

                // A code ref used as FILTER $var is already a filter (Filters::fetch).
                let filter_fn: FilterFn = if let Value::Function(func) = &name_val {
                    let func = Rc::clone(func);
                    Rc::new(move |text: &str| {
                        func.call(&[Value::Str(text.to_string())], CallContext::Scalar)
                            .map(|v| v.to_tt_string())
                    })
                } else {
                    self.filter(
                        &filter_name,
                        if pos_vals.is_empty() {
                            None
                        } else {
                            Some(&pos_vals)
                        },
                        alias_str.as_deref(),
                    )?
                };

                let mut sub_output = String::new();
                self.eval_block(block, &mut sub_output)?;
                let filtered = filter_fn.filter(&sub_output)?;
                output.push_str(&filtered);
                Ok(())
            }
            Stmt::If {
                branches,
                else_block,
            } => {
                for (cond, blk) in branches {
                    let cval = self.eval_expr(cond)?;
                    if cval.is_truthy() {
                        return self.eval_block(blk, output);
                    }
                }
                if let Some(eblk) = else_block {
                    self.eval_block(eblk, output)?;
                }
                Ok(())
            }
            Stmt::Switch {
                expr,
                cases,
                default_block,
            } => {
                let result_val = self.eval_expr(expr)?;
                let result_str = result_val.to_tt_string();

                for (match_expr, blk) in cases {
                    let mval = self.eval_expr(match_expr)?;
                    let matched = match mval {
                        Value::List(l_rc) => l_rc
                            .borrow()
                            .iter()
                            .any(|item| item.to_tt_string() == result_str),
                        other => other.to_tt_string() == result_str,
                    };
                    if matched {
                        return self.eval_block(blk, output);
                    }
                }
                if let Some(dblk) = default_block {
                    self.eval_block(dblk, output)?;
                }
                Ok(())
            }
            Stmt::Foreach {
                loop_var,
                list,
                args: _,
                block,
                label,
            } => {
                let list_val = self.eval_expr(list)?;
                let iter_rc: Rc<TemplateIterator> = match &list_val {
                    Value::Object(obj) => {
                        if let Some(existing) = obj.as_any().downcast_ref::<TemplateIterator>() {
                            Rc::new(existing.clone())
                        } else {
                            Rc::new(TemplateIterator::new(&list_val))
                        }
                    }
                    _ => Rc::new(TemplateIterator::new(&list_val)),
                };

                let my_label = label.as_deref().unwrap_or("LOOP");

                let old_loop = if loop_var.is_some() {
                    self.stash().get(&Value::Str("loop".to_string()), None).ok()
                } else {
                    self.localise(None);
                    None
                };

                let mut stash = self.stash();
                let _ = stash.set(
                    &Value::Str("loop".to_string()),
                    Value::Object(Rc::clone(&iter_rc) as Rc<dyn crate::value::TemplateObject>),
                    false,
                );

                let (mut cur_val, mut cur_err) = iter_rc.get_first();

                let loop_res: Result<(), ControlFlow> = (|| {
                    while cur_err.is_none() {
                        if let Some(ref target_name) = loop_var {
                            stash.set_raw(target_name.clone(), cur_val.clone());
                        } else if matches!(cur_val, Value::Hash(_)) {
                            let pairs =
                                vec![(Value::Str("import".to_string()), vec![cur_val.clone()])];
                            let _ = stash.get_pairs(&pairs, None);
                        }

                        match self.eval_block(block, output) {
                            Ok(()) => {}
                            Err(ControlFlow::Next(ref lbl))
                                if lbl.is_none() || lbl.as_deref() == Some(my_label) =>
                            {
                                let next_pair = iter_rc.get_next();
                                cur_val = next_pair.0;
                                cur_err = next_pair.1;
                                continue;
                            }
                            Err(ControlFlow::Last(ref lbl))
                                if lbl.is_none() || lbl.as_deref() == Some(my_label) =>
                            {
                                break;
                            }
                            Err(other) => return Err(other),
                        }

                        let next_pair = iter_rc.get_next();
                        cur_val = next_pair.0;
                        cur_err = next_pair.1;
                    }
                    Ok(())
                })();

                if loop_var.is_some() {
                    let restore_val = old_loop.unwrap_or(Value::Undefined);
                    let _ = stash.set(&Value::Str("loop".to_string()), restore_val, false);
                } else {
                    self.delocalise();
                }

                loop_res?;

                if let Some(code) = cur_err {
                    if code != STATUS_DONE {
                        return Err(ControlFlow::Exception(TemplateException::new(
                            ERROR_UNDEF,
                            format!("iterator error: {}", code),
                        )));
                    }
                }

                Ok(())
            }
            Stmt::While { expr, block, label } => {
                let my_label = label.as_deref().unwrap_or("LOOP");
                let while_max = get_while_max();
                let mut failsafe = while_max as isize;

                loop {
                    let cond = self.eval_expr(expr)?;
                    if !cond.is_truthy() {
                        break;
                    }
                    failsafe -= 1;
                    if failsafe < 0 {
                        break;
                    }
                    match self.eval_block(block, output) {
                        Ok(()) => {}
                        Err(ControlFlow::Next(ref lbl))
                            if lbl.is_none() || lbl.as_deref() == Some(my_label) =>
                        {
                            continue;
                        }
                        Err(ControlFlow::Last(ref lbl))
                            if lbl.is_none() || lbl.as_deref() == Some(my_label) =>
                        {
                            break;
                        }
                        Err(other) => return Err(other),
                    }
                }

                if failsafe < 0 {
                    return Err(ControlFlow::Exception(TemplateException::new(
                        ERROR_UNDEF,
                        format!("WHILE loop terminated (> {} iterations)\n", while_max),
                    )));
                }

                Ok(())
            }
            Stmt::Try {
                block,
                catches,
                final_block,
            } => {
                let mut try_output = String::new();
                let mut active_error: Option<TemplateException> = None;

                if let Err(cf) = self.eval_block(block, &mut try_output) {
                    let ex = self.catch_control_flow(cf, &mut try_output)?;
                    if ex.type_() == ERROR_RETURN || ex.type_() == "stop" {
                        return Err(ControlFlow::Exception(ex));
                    }

                    let caught_text = ex.text.clone();
                    let ex_val = Value::from(ex.clone());
                    let stash = self.stash();
                    let _ = stash.set(&Value::Str("error".to_string()), ex_val.clone(), false);
                    let _ = stash.set(&Value::Str("e".to_string()), ex_val, false);

                    // Collect named handlers vs default handler (first untyped/DEFAULT catch wins)
                    let mut handler_names: Vec<&str> = Vec::new();
                    let mut default_catch: Option<&Block> = None;
                    for (maybe_name, cblk) in catches {
                        match maybe_name {
                            Some(n) => handler_names.push(n.as_str()),
                            None => {
                                if default_catch.is_none() {
                                    default_catch = Some(cblk);
                                }
                            }
                        }
                    }

                    let chosen_block: Option<&Block> =
                        if let Some(matched_name) = ex.select_handler(&handler_names) {
                            catches
                                .iter()
                                .find(|(n, _)| n.as_deref() == Some(matched_name))
                                .map(|(_, b)| b)
                        } else {
                            default_catch
                        };

                    if let Some(cblk) = chosen_block {
                        if let Err(mut catch_cf) = self.eval_block(cblk, &mut try_output) {
                            // A rethrown caught exception shares its text buffer with
                            // `try_output` in Perl, so it must not be appended again.
                            if let ControlFlow::Exception(ref mut e) = catch_cf {
                                if !e.text.is_empty() && e.text == caught_text {
                                    e.text = String::new();
                                }
                            }
                            let new_ex = self.catch_control_flow(catch_cf, &mut try_output)?;
                            active_error = Some(new_ex);
                        }
                    } else {
                        active_error = Some(ex);
                    }
                }

                if let Some(fblk) = final_block {
                    if let Err(final_cf) = self.eval_block(fblk, &mut try_output) {
                        let final_ex = self.catch_control_flow(final_cf, &mut try_output)?;
                        active_error = Some(final_ex);
                    }
                }

                if let Some(mut err) = active_error {
                    err.text = try_output;
                    return Err(ControlFlow::Exception(err));
                }

                output.push_str(&try_output);
                Ok(())
            }
            Stmt::Use { names, args, alias } => {
                let name_val = if let Some(first) = names.first() {
                    self.eval_expr(first)?
                } else {
                    Value::Str(String::new())
                };
                let plugin_name = name_val.to_tt_string();

                let mut pos_vals = Vec::with_capacity(args.positional.len() + 1);
                for p in &args.positional {
                    pos_vals.push(self.eval_expr(p)?);
                }
                if let Some(named_map) = self.eval_named_args(&args.named)? {
                    pos_vals.push(Value::from_hash(named_map));
                }

                let plugin_val = self
                    .plugin(&plugin_name, &pos_vals)
                    .map_err(ControlFlow::Exception)?;

                if let Some(target) = alias {
                    self.eval_assign(target, plugin_val, false)?;
                } else if plugin_name.contains('.') {
                    let nodes: Vec<IdentNode> = plugin_name
                        .split('.')
                        .map(|part| IdentNode {
                            key: Expr::Str(part.to_string()),
                            args: None,
                        })
                        .collect();
                    self.eval_assign(&AssignTarget::Ident(nodes), plugin_val, false)?;
                } else {
                    let _ = self
                        .stash()
                        .set(&Value::Str(plugin_name), plugin_val, false)?;
                }

                Ok(())
            }
            Stmt::View {
                names,
                args,
                block,
                defblocks,
            } => {
                let name_val = if let Some(first) = names.first() {
                    self.eval_expr(first)?
                } else {
                    Value::Str(String::new())
                };
                let mut config = self.eval_named_args(&args.named)?.unwrap_or_default();
                if !defblocks.is_empty() {
                    let mut bmap = HashMap::new();
                    for (bname, bblk) in defblocks {
                        let doc = Rc::new(Document::from_block(bblk.clone(), Some(bname.clone())));
                        bmap.insert(
                            bname.clone(),
                            Value::Object(doc as Rc<dyn crate::value::TemplateObject>),
                        );
                    }
                    config.insert("blocks".to_string(), Value::from_hash(bmap));
                }

                let stash = self.stash();
                let old_view = stash
                    .get(&Value::Str("view".to_string()), None)
                    .unwrap_or(Value::Undefined);
                let view_obj = self.view(config).map_err(ControlFlow::Exception)?;
                let view_val = Value::Object(
                    Rc::new(view_obj.clone()) as Rc<dyn crate::value::TemplateObject>
                );

                if name_val.to_tt_string().contains('.') {
                    let nodes: Vec<IdentNode> = name_val
                        .to_tt_string()
                        .split('.')
                        .map(|part| IdentNode {
                            key: Expr::Str(part.to_string()),
                            args: None,
                        })
                        .collect();
                    self.eval_assign(&AssignTarget::Ident(nodes), view_val.clone(), false)?;
                } else {
                    let _ = stash.set(&name_val, view_val.clone(), false)?;
                }
                let _ = stash.set(&Value::Str("view".to_string()), view_val, false)?;

                let mut discarded_output = String::new();
                let res = self.eval_block(block, &mut discarded_output);

                let _ = stash.set(&Value::Str("view".to_string()), old_view, false)?;
                view_obj.seal();

                res
            }
            Stmt::NoPerl => Err(ControlFlow::Exception(TemplateException::new(
                ERROR_PERL,
                "EVAL_PERL not set",
            ))),
            Stmt::AnonBlock(block) => {
                let s = self.eval_template_block(block)?;
                output.push_str(&s);
                Ok(())
            }
            Stmt::Capture { target, block } => {
                let mut sub_output = String::new();
                self.eval_block(block, &mut sub_output)?;
                self.eval_assign(target, Value::Str(sub_output), false)?;
                Ok(())
            }
            Stmt::Macro {
                ident,
                params,
                block,
            } => {
                let ctx = self.clone();
                let blk = block.clone();
                let param_names = params.clone();
                let callable = MacroCallable {
                    context: ctx,
                    params: param_names,
                    block: blk,
                };
                let _ = self.stash().set(
                    &Value::Str(ident.clone()),
                    Value::Function(Rc::new(callable)),
                    false,
                )?;
                Ok(())
            }
        }
    }

    fn eval_filenames(&self, files: &[Expr]) -> Result<Value, ControlFlow> {
        if files.len() == 1 {
            self.eval_expr(&files[0])
        } else {
            let mut list = Vec::with_capacity(files.len());
            for f in files {
                list.push(self.eval_expr(f)?);
            }
            Ok(Value::from_list(list))
        }
    }

    fn eval_named_args(
        &self,
        named: &[NamedArg],
    ) -> Result<Option<HashMap<String, Value>>, ControlFlow> {
        if named.is_empty() {
            return Ok(None);
        }
        let mut map = HashMap::with_capacity(named.len());
        for narg in named {
            match narg {
                NamedArg::KeyValue(k_expr, v_expr) => {
                    let k = self.eval_expr(k_expr)?.to_tt_string();
                    let v = self.eval_expr(v_expr)?;
                    map.insert(k, v);
                }
                NamedArg::Assign(nodes, v_expr) => {
                    let v = self.eval_expr(v_expr)?;
                    if nodes.len() == 1 && nodes[0].args.is_none() {
                        let k = self.eval_expr(&nodes[0].key)?.to_tt_string();
                        map.insert(k, v);
                    } else {
                        self.eval_assign(&AssignTarget::Ident(nodes.clone()), v, false)?;
                    }
                }
            }
        }
        Ok(Some(map))
    }

    fn eval_ident_pairs(
        &self,
        nodes: &[IdentNode],
    ) -> Result<Vec<(Value, Vec<Value>)>, ControlFlow> {
        let mut pairs = Vec::with_capacity(nodes.len());
        for node in nodes {
            let key_val = self.eval_expr(&node.key)?;
            let mut arg_vals = Vec::new();
            if let Some(ref args_spec) = node.args {
                for p in &args_spec.positional {
                    arg_vals.push(self.eval_expr(p)?);
                }
                if let Some(named_map) = self.eval_named_args(&args_spec.named)? {
                    arg_vals.push(Value::from_hash(named_map));
                }
            }
            pairs.push((key_val, arg_vals));
        }
        Ok(pairs)
    }

    fn eval_assign(
        &self,
        target: &AssignTarget,
        val: Value,
        is_default: bool,
    ) -> Result<Value, ControlFlow> {
        let stash = self.stash();
        match target {
            AssignTarget::Expr(e) => {
                let k = self.eval_expr(e)?;
                stash.set(&k, val, is_default)
            }
            AssignTarget::Ident(nodes) => {
                if nodes.len() == 1 && nodes[0].args.is_none() {
                    let k = self.eval_expr(&nodes[0].key)?;
                    stash.set(&k, val, is_default)
                } else {
                    let pairs = self.eval_ident_pairs(nodes)?;
                    stash.set_pairs(&pairs, val, is_default)
                }
            }
        }
    }

    pub fn eval_expr(&self, expr: &Expr) -> Result<Value, ControlFlow> {
        match expr {
            Expr::Str(s) => Ok(Value::Str(s.clone())),
            Expr::Int(i) => Ok(Value::Int(*i)),
            Expr::Float(f) => Ok(Value::Float(*f)),
            Expr::Quoted(items) => {
                let mut s = String::new();
                for item in items {
                    let v = self.eval_expr(item)?;
                    s.push_str(&v.to_tt_string());
                }
                Ok(Value::Str(s))
            }
            Expr::Ident(nodes) => {
                if nodes.is_empty() {
                    return Ok(Value::Str(String::new()));
                }
                let stash = self.stash();
                if nodes.len() == 1 && nodes[0].args.is_none() {
                    let k = self.eval_expr(&nodes[0].key)?;
                    stash.get(&k, None)
                } else {
                    let pairs = self.eval_ident_pairs(nodes)?;
                    stash.get_pairs(&pairs, None)
                }
            }
            Expr::IdentRef(nodes) => {
                if nodes.is_empty() {
                    return Ok(Value::Str(String::new()));
                }
                let stash = self.stash();
                let pairs = self.eval_ident_pairs(nodes)?;
                stash.getref_pairs(&pairs)
            }
            Expr::List(items) => {
                let mut list = Vec::with_capacity(items.len());
                for item in items {
                    match item {
                        Expr::Range(start_expr, end_expr) => {
                            let range_vals = self.eval_range_items(start_expr, end_expr)?;
                            list.extend(range_vals);
                        }
                        other => {
                            let val = self.eval_expr(other)?;
                            list.push(val);
                        }
                    }
                }
                Ok(Value::from_list(list))
            }
            Expr::Range(start_expr, end_expr) => {
                let items = self.eval_range_items(start_expr, end_expr)?;
                Ok(Value::from_list(items))
            }
            Expr::Hash(entries) => {
                let mut map = HashMap::with_capacity(entries.len());
                for (k_expr, v_expr) in entries {
                    let k = self.eval_expr(k_expr)?.to_tt_string();
                    let v = self.eval_expr(v_expr)?;
                    map.insert(k, v);
                }
                Ok(Value::from_hash(map))
            }
            Expr::Paren(inner) => self.eval_expr(inner),
            Expr::Not(inner) => {
                let v = self.eval_expr(inner)?;
                if v.is_truthy() {
                    Ok(Value::Str(String::new()))
                } else {
                    Ok(Value::Int(1))
                }
            }
            Expr::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                let c = self.eval_expr(cond)?;
                if c.is_truthy() {
                    self.eval_expr(then_expr)
                } else {
                    self.eval_expr(else_expr)
                }
            }
            Expr::Assign { target, expr } => {
                let val = self.eval_expr(expr)?;
                self.eval_assign(target, val, false)
            }
            Expr::BinOp { op, left, right } => self.eval_binop(*op, left, right),
        }
    }

    fn eval_range_items(
        &self,
        start_expr: &Expr,
        end_expr: &Expr,
    ) -> Result<Vec<Value>, ControlFlow> {
        let s_val = self.eval_expr(start_expr)?;
        let e_val = self.eval_expr(end_expr)?;
        let s_str = s_val.to_tt_string();
        let e_str = e_val.to_tt_string();

        // Check if both are integers or numeric
        let (s_num, s_is_num) = s_val.to_number_checked();
        let (e_num, e_is_num) = e_val.to_number_checked();

        if (matches!(s_val, Value::Int(_) | Value::Float(_)) || s_is_num)
            && (matches!(e_val, Value::Int(_) | Value::Float(_)) || e_is_num)
        {
            let start_i = s_num.trunc() as i64;
            let end_i = e_num.trunc() as i64;
            if start_i <= end_i {
                let count = (end_i - start_i + 1).min(100_000) as usize;
                let mut out = Vec::with_capacity(count);
                for i in start_i..=end_i {
                    out.push(Value::Int(i));
                    if out.len() >= 100_000 {
                        break;
                    }
                }
                return Ok(out);
            } else {
                return Ok(Vec::new());
            }
        }

        // Perl magical string range `a..z`
        if s_str.len() == 1 && e_str.len() == 1 {
            let sc = s_str.chars().next().unwrap();
            let ec = e_str.chars().next().unwrap();
            if (sc.is_ascii_lowercase() && ec.is_ascii_lowercase() && sc <= ec)
                || (sc.is_ascii_uppercase() && ec.is_ascii_uppercase() && sc <= ec)
            {
                return Ok((sc..=ec).map(|c| Value::Str(c.to_string())).collect());
            }
        }

        Ok(Vec::new())
    }

    fn division_by_zero_error(&self) -> ControlFlow {
        let inner = self.inner.borrow();
        let line_start = inner.current_line.split('-').next().unwrap_or("1");
        let suffix = if inner.current_from_compiled {
            ", <DATA> chunk 1."
        } else {
            "."
        };
        ControlFlow::Exception(TemplateException::new(
            ERROR_UNDEF,
            format!(
                "Illegal division by zero at {} line {}{}",
                inner.current_file, line_start, suffix
            ),
        ))
    }

    fn eval_binop(&self, op: BinOp, left: &Expr, right: &Expr) -> Result<Value, ControlFlow> {
        match op {
            BinOp::And => {
                let l = self.eval_expr(left)?;
                if !l.is_truthy() {
                    Ok(l)
                } else {
                    self.eval_expr(right)
                }
            }
            BinOp::Or => {
                let l = self.eval_expr(left)?;
                if l.is_truthy() {
                    Ok(l)
                } else {
                    self.eval_expr(right)
                }
            }
            BinOp::Cat => {
                let l = self.eval_expr(left)?;
                let r = self.eval_expr(right)?;
                Ok(Value::Str(format!(
                    "{}{}",
                    l.to_tt_string(),
                    r.to_tt_string()
                )))
            }
            BinOp::Add => {
                let l = self.eval_expr(left)?.to_number();
                let r = self.eval_expr(right)?.to_number();
                Ok(Self::number_to_value(l + r))
            }
            BinOp::Sub => {
                let l = self.eval_expr(left)?.to_number();
                let r = self.eval_expr(right)?.to_number();
                Ok(Self::number_to_value(l - r))
            }
            BinOp::Mul => {
                let l = self.eval_expr(left)?.to_number();
                let r = self.eval_expr(right)?.to_number();
                Ok(Self::number_to_value(l * r))
            }
            BinOp::Div => {
                let l = self.eval_expr(left)?.to_number();
                let r = self.eval_expr(right)?.to_number();
                if r == 0.0 {
                    return Err(self.division_by_zero_error());
                }
                Ok(Self::number_to_value(l / r))
            }
            BinOp::IntDiv => {
                let l = self.eval_expr(left)?.to_number();
                let r = self.eval_expr(right)?.to_number();
                if r == 0.0 {
                    return Err(self.division_by_zero_error());
                }
                let q = (l / r).trunc();
                Ok(Value::Int(q as i64))
            }
            BinOp::Mod => {
                let l = self.eval_expr(left)?.to_int();
                let r = self.eval_expr(right)?.to_int();
                if r == 0 {
                    return Err(self.division_by_zero_error());
                }
                // Perl's `%` operator uses Euclidean-like floored modulo with sign of divisor
                let rem = ((l % r) + r) % r;
                Ok(Value::Int(rem))
            }
            BinOp::Eq => {
                let l = self.eval_expr(left)?;
                let r = self.eval_expr(right)?;
                Ok(if l.seq(&r) {
                    Value::Int(1)
                } else {
                    Value::Str(String::new())
                })
            }
            BinOp::Ne => {
                let l = self.eval_expr(left)?;
                let r = self.eval_expr(right)?;
                Ok(if !l.seq(&r) {
                    Value::Int(1)
                } else {
                    Value::Str(String::new())
                })
            }
            BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                let l = self.eval_expr(left)?;
                let r = self.eval_expr(right)?;
                let ln = l.to_number();
                let rn = r.to_number();
                let res = match op {
                    BinOp::Lt => ln < rn,
                    BinOp::Gt => ln > rn,
                    BinOp::Le => ln <= rn,
                    BinOp::Ge => ln >= rn,
                    _ => unreachable!(),
                };
                Ok(if res {
                    Value::Int(1)
                } else {
                    Value::Str(String::new())
                })
            }
        }
    }

    fn number_to_value(n: f64) -> Value {
        if n.fract() == 0.0 && n >= (i64::MIN as f64) && n <= (i64::MAX as f64) {
            Value::Int(n as i64)
        } else {
            Value::Str(format_perl_float(n))
        }
    }
}

struct MacroCallable {
    context: Context,
    params: Option<Vec<String>>,
    block: Block,
}

impl Callable for MacroCallable {
    fn call(&self, args: &[Value], _ctx: CallContext) -> Result<Value, ControlFlow> {
        let mut local_params = HashMap::new();
        if let Some(ref pnames) = self.params {
            let n = pnames.len();
            for (i, pname) in pnames.iter().enumerate() {
                let val = args.get(i).cloned().unwrap_or(Value::Undefined);
                local_params.insert(pname.clone(), val);
            }
            if let Some(Value::Hash(h_rc)) = args.get(n) {
                for (k, v) in h_rc.borrow().iter() {
                    local_params.insert(k.clone(), v.clone());
                }
            }
        } else if let Some(Value::Hash(h_rc)) = args.first() {
            for (k, v) in h_rc.borrow().iter() {
                local_params.insert(k.clone(), v.clone());
            }
        }

        self.context.localise(Some(local_params));
        let mut output = String::new();
        let res = self.context.eval_block(&self.block, &mut output);
        self.context.delocalise();

        match res {
            Ok(()) => Ok(Value::Str(output)),
            Err(cf) => {
                // Note: in Directive::macro, `die $@ if $@;` propagates `return` with macro's output
                match cf {
                    ControlFlow::Exception(mut ex) => {
                        if ex.text().is_empty() && !output.is_empty() {
                            ex.text = output;
                        }
                        Err(ControlFlow::Exception(ex))
                    }
                    other => Err(other),
                }
            }
        }
    }
}
