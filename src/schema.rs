//! Infer a JSON Schema for a template's input variables, and list the
//! plugin modules (`USE`) and the non-standard filters it needs.
//!
//! The scanner walks the parsed AST in order, keeping a scope of the names
//! the template defines itself (`SET`, `FOREACH` loop variables, `MACRO`
//! parameters, `INCLUDE` arguments, `USE` plugins, ...).  Every other name is
//! an input, and how it is used shapes its schema:
//!
//! * `foo.bar` makes `foo` an object with a `bar` property, `foo.0` and
//!   `FOREACH x IN foo` make it an array (and `x` describes its items),
//!   `foo.$key` gives it `additionalProperties`.
//! * Virtual methods hint at types: `.keys` and `.pairs` want an object,
//!   `.first` and `.join` an array, `.upper` and `.length` a scalar.
//!   TT looks up a hash key before a vmethod of the same name; the scanner
//!   cannot know the data, so it reads a known vmethod name as one, except
//!   for common field names (`items`, `item`, `hash`, `list`) used without
//!   parentheses.
//! * Printing a value wants a scalar, arithmetic wants a number, and the
//!   arguments of built-in filters have their types (`truncate(len)`).
//! * A name is required when it is used unconditionally; uses inside `IF`
//!   tests and branches, loops, `||`/`&&` and `DEFAULT` leave it optional.
//!
//! `INCLUDE`, `PROCESS` and `WRAPPER` with literal names are followed
//! (BLOCKs first, then through a loader such as the template's providers);
//! names computed at run time are reported as warnings.  TT coerces types
//! freely, so the result is a well-founded guess rather than a proof.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    rc::Rc,
};

use serde_json::{json, Map, Value as Json};

use crate::{
    document::Document,
    error::TemplateException,
    filters::Filters,
    parser::ast::{
        ArgsSpec, AssignTarget, BinOp, Block, Expr, IdentNode, NamedArg, SpannedStmt, Stmt,
    },
    Template,
};

const OBJECT: u8 = 1;
const ARRAY: u8 = 2;
const STRING: u8 = 4;
const NUMBER: u8 = 8;
const SCALAR: u8 = STRING | NUMBER;
const ANY: u8 = OBJECT | ARRAY | STRING | NUMBER;

/// Variables TT provides itself.
const BUILTINS: &[&str] = &["template", "component"];

/// Deepest INCLUDE/PROCESS/MACRO nesting the scanner follows.
const MAX_DEPTH: usize = 32;

/// Options for [`infer`].
#[derive(Debug, Clone, Default)]
pub struct SchemaOptions {
    /// Record where each variable is used, as `x-tt-source: ["file:line"]`.
    pub sources: bool,
    /// The schema's `title`.
    pub title: Option<String>,
}

/// The inferred schema, and anything the scanner could not follow.
#[derive(Debug, Clone)]
pub struct Inference {
    pub schema: Json,
    /// Plugin modules loaded with `USE`.
    pub modules: BTreeSet<String>,
    /// Filters applied that are neither built in nor aliases the templates
    /// define themselves (`FILTER name = ...`): the ones a host must supply.
    pub filters: BTreeSet<String>,
    pub warnings: Vec<String>,
}

impl Inference {
    /// The three results as one JSON object: `variables` (the schema),
    /// `modules` and `filters`.
    pub fn report(&self) -> Json {
        json!({ "variables": self.schema, "modules": self.modules, "filters": self.filters })
    }
}

/// Loads a template by name for `INCLUDE`, `PROCESS` and `WRAPPER`.
pub type Loader<'a> = dyn FnMut(&str) -> Result<Rc<Document>, String> + 'a;

/// Infer the input schema of `doc`, named `name` in warnings and sources.
/// Without a loader, only BLOCKs defined in the templates are followed.
pub fn infer(
    doc: &Document,
    name: &str,
    loader: Option<&mut Loader<'_>>,
    opts: &SchemaOptions,
) -> Inference {
    let mut scanner = Scanner::new(loader, opts.sources);
    scanner.main(doc, name);
    let mut schema = Map::new();
    schema.insert(
        "$schema".into(),
        json!("https://json-schema.org/draft/2020-12/schema"),
    );
    if let Some(title) = &opts.title {
        schema.insert("title".into(), json!(title));
    }
    schema.insert("type".into(), json!("object"));
    if let Json::Object(body) = scanner.emit(0, 0) {
        schema.extend(body.into_iter().filter(|(k, _)| k != "type"));
    }
    let filters = scanner
        .filters
        .difference(&scanner.filter_aliases)
        .filter(|name| !Filters::is_builtin(name))
        .cloned()
        .collect();
    Inference {
        schema: Json::Object(schema),
        modules: scanner.modules,
        filters,
        warnings: scanner.warnings,
    }
}

impl Template {
    /// Infer the input schema of the named template, loading it and the
    /// templates it includes through this Template's providers.
    pub fn infer_schema(
        &self,
        name: &str,
        opts: &SchemaOptions,
    ) -> Result<Inference, TemplateException> {
        let doc = self.context().template(name)?;
        Ok(self.infer_schema_doc(&doc, name, opts))
    }

    /// Infer the input schema of template source text.
    pub fn infer_schema_text(
        &self,
        text: &str,
        opts: &SchemaOptions,
    ) -> Result<Inference, TemplateException> {
        let doc = self.context().compile_text(text)?;
        Ok(self.infer_schema_doc(&doc, "input text", opts))
    }

    fn infer_schema_doc(&self, doc: &Document, name: &str, opts: &SchemaOptions) -> Inference {
        let context = self.context();
        let mut loader = |name: &str| context.template(name).map_err(|e| e.to_string());
        infer(doc, name, Some(&mut loader), opts)
    }
}

type NodeId = usize;

/// What is known about one value: the inputs hang off node 0.
struct Node {
    props: BTreeMap<String, NodeId>,
    required: BTreeSet<String>,
    items: Option<NodeId>,
    any_key: Option<NodeId>,
    /// Types every use allows, and types some use allows.
    inter: u8,
    union: u8,
    callable: bool,
    /// Assigned or defaulted by the template, so never required.
    optional: bool,
    default: Option<Json>,
    sources: BTreeSet<String>,
}

impl Node {
    fn new() -> Self {
        Self {
            props: BTreeMap::new(),
            required: BTreeSet::new(),
            items: None,
            any_key: None,
            inter: ANY,
            union: 0,
            callable: false,
            optional: false,
            default: None,
            sources: BTreeSet::new(),
        }
    }
}

struct MacroDef {
    params: Vec<String>,
    block: Block,
}

#[derive(Clone)]
enum Binding {
    /// `base` is the conditional depth the name was bound at: uses at that
    /// depth are unconditional relative to the binding.
    Value {
        node: NodeId,
        base: usize,
    },
    Macro(Rc<MacroDef>),
}

type BlockMap = Rc<HashMap<String, Rc<Block>>>;

struct Scanner<'l, 'a> {
    nodes: Vec<Node>,
    frames: Vec<HashMap<String, Binding>>,
    /// How many conditionals enclose the current statement.
    cond: usize,
    /// The BLOCKs of the templates being scanned, innermost last (TT's
    /// BLKSTACK).
    blocks: Vec<BlockMap>,
    /// The context's BLOCKS: those of every template processed without
    /// localising, the main one included.  TT keeps them for the rest of
    /// the request, so a BLOCK library PROCESSed (even from inside an
    /// INCLUDE) serves the templates after it.
    context_blocks: HashMap<String, Rc<Block>>,
    loader: Option<&'l mut Loader<'a>>,
    docs: HashMap<String, Option<(Rc<Document>, BlockMap)>>,
    /// Templates, BLOCKs and MACROs being scanned, to stop recursion.
    stack: Vec<String>,
    seen: HashSet<String>,
    macros: Vec<Rc<MacroDef>>,
    modules: BTreeSet<String>,
    filters: BTreeSet<String>,
    filter_aliases: BTreeSet<String>,
    warnings: Vec<String>,
    file: String,
    line: Option<String>,
    sources: bool,
}

fn literal(e: &Expr) -> Option<Json> {
    match e {
        Expr::Str(s) => Some(json!(s)),
        Expr::Int(i) => Some(json!(i)),
        Expr::Float(f) => Some(json!(f)),
        _ => None,
    }
}

fn target_name(t: &AssignTarget) -> Option<&str> {
    match t {
        AssignTarget::Expr(Expr::Str(s)) => Some(s),
        AssignTarget::Ident(path) if path.len() == 1 && path[0].args.is_none() => {
            match &path[0].key {
                Expr::Str(s) => Some(s),
                _ => None,
            }
        }
        _ => None,
    }
}

/// TT hides keys starting with `_` or `.`.
fn is_private(key: &str) -> bool {
    key.starts_with('_') || key.starts_with('.')
}

fn block_map(doc: &Document) -> BlockMap {
    Rc::new(
        doc.defblocks
            .iter()
            .map(|(k, b)| (k.clone(), Rc::new(b.clone())))
            .collect(),
    )
}

impl<'l, 'a> Scanner<'l, 'a> {
    fn new(loader: Option<&'l mut Loader<'a>>, sources: bool) -> Self {
        Self {
            nodes: vec![Node::new()],
            frames: vec![HashMap::new()],
            cond: 0,
            blocks: Vec::new(),
            context_blocks: HashMap::new(),
            loader,
            docs: HashMap::new(),
            stack: Vec::new(),
            seen: HashSet::new(),
            macros: Vec::new(),
            modules: BTreeSet::new(),
            filters: BTreeSet::new(),
            filter_aliases: BTreeSet::new(),
            warnings: Vec::new(),
            file: String::new(),
            line: None,
            sources,
        }
    }

    fn main(&mut self, doc: &Document, name: &str) {
        let blocks = block_map(doc);
        self.file = name.to_string();
        self.blocks.push(blocks.clone());
        self.context_blocks
            .extend(blocks.iter().map(|(k, b)| (k.clone(), b.clone())));
        self.stack.push(format!("file:{name}"));
        self.block(&doc.block);

        // BLOCKs and MACROs never used here may be used by other templates:
        // what they read is optional.
        self.cond += 1;
        let mut unused: Vec<_> = blocks
            .iter()
            .filter(|(_, b)| !self.seen.contains(&Self::block_key(b)))
            .collect();
        unused.sort_by(|a, b| a.0.cmp(b.0));
        for (name, b) in unused {
            let b = b.clone();
            self.run(Self::block_key(&b), name, |s| s.block(&b));
        }
        let macros: Vec<_> = self
            .macros
            .iter()
            .filter(|m| !self.seen.contains(&Self::macro_key(m)))
            .cloned()
            .collect();
        for m in macros {
            self.call_macro(&m, None);
        }
        self.cond -= 1;
    }

    // --- nodes ---

    fn fresh(&mut self, kinds: u8) -> NodeId {
        self.nodes.push(Node::new());
        let id = self.nodes.len() - 1;
        self.hint(id, kinds);
        id
    }

    fn hint(&mut self, id: NodeId, kinds: u8) {
        if kinds != 0 {
            let n = &mut self.nodes[id];
            n.inter &= kinds;
            n.union |= kinds;
        }
    }

    fn note_source(&mut self, id: NodeId) {
        if self.sources {
            let at = match &self.line {
                Some(line) => format!("{}:{}", self.file, line),
                None => self.file.clone(),
            };
            self.nodes[id].sources.insert(at);
        }
    }

    fn child(&mut self, parent: NodeId, key: &str, required: bool) -> NodeId {
        self.hint(parent, OBJECT);
        let id = match self.nodes[parent].props.get(key) {
            Some(&id) => id,
            None => {
                let id = self.fresh(0);
                self.nodes[parent].props.insert(key.to_string(), id);
                id
            }
        };
        if required {
            self.nodes[parent].required.insert(key.to_string());
        }
        self.note_source(id);
        id
    }

    fn items(&mut self, parent: NodeId) -> NodeId {
        self.hint(parent, ARRAY);
        match self.nodes[parent].items {
            Some(id) => id,
            None => {
                let id = self.fresh(0);
                self.nodes[parent].items = Some(id);
                id
            }
        }
    }

    fn any_key(&mut self, parent: NodeId) -> NodeId {
        self.hint(parent, OBJECT);
        match self.nodes[parent].any_key {
            Some(id) => id,
            None => {
                let id = self.fresh(0);
                self.nodes[parent].any_key = Some(id);
                id
            }
        }
    }

    /// A fresh array whose items have the given types.
    fn list_of(&mut self, kinds: u8) -> NodeId {
        let list = self.fresh(ARRAY);
        let item = self.items(list);
        self.hint(item, kinds);
        list
    }

    /// The input `name`, assigned or defaulted by the template.
    fn optional_input(&mut self, name: &str) -> NodeId {
        let id = self.child(0, name, false);
        self.nodes[id].optional = true;
        id
    }

    // --- scope ---

    fn lookup(&self, name: &str) -> Option<Binding> {
        self.frames.iter().rev().find_map(|f| f.get(name).cloned())
    }

    fn bind(&mut self, name: &str, binding: Binding) {
        self.frames
            .last_mut()
            .unwrap()
            .insert(name.to_string(), binding);
    }

    fn bind_node(&mut self, name: &str, node: NodeId) {
        self.bind(
            name,
            Binding::Value {
                node,
                base: self.cond,
            },
        );
    }

    fn unbind(&mut self, name: &str) -> Option<Binding> {
        self.frames.last_mut().unwrap().remove(name)
    }

    fn restore(&mut self, name: &str, saved: Option<Binding>) {
        match saved {
            Some(b) => self.bind(name, b),
            None => {
                self.unbind(name);
            }
        }
    }

    fn warn(&mut self, msg: impl std::fmt::Display) {
        let at = match &self.line {
            Some(line) => format!("{} line {}", self.file, line),
            None => self.file.clone(),
        };
        self.warnings.push(format!("{at}: {msg}"));
    }

    /// Scan `f` as the template, BLOCK or MACRO identified by `key`, unless
    /// that would recurse.
    fn run(&mut self, key: String, label: &str, f: impl FnOnce(&mut Self)) {
        if self.stack.contains(&key) {
            return;
        }
        if self.stack.len() >= MAX_DEPTH {
            self.warn(format_args!("{label}: nested too deeply to follow"));
            return;
        }
        self.seen.insert(key.clone());
        self.stack.push(key);
        f(self);
        self.stack.pop();
    }

    fn block_key(b: &Rc<Block>) -> String {
        format!("block:{:p}", Rc::as_ptr(b))
    }

    fn macro_key(m: &Rc<MacroDef>) -> String {
        format!("macro:{:p}", Rc::as_ptr(m))
    }

    /// Scan `f` as a test or branch: what it reads is optional.
    fn conditional<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        self.cond += 1;
        let out = f(self);
        self.cond -= 1;
        out
    }

    // --- statements ---

    fn block(&mut self, b: &Block) {
        for s in &b.stmts {
            self.stmt(s);
        }
    }

    fn stmt(&mut self, s: &SpannedStmt) {
        if s.line.is_some() {
            self.line = s.line.clone();
        }
        match &s.stmt {
            Stmt::Text(_)
            | Stmt::Return
            | Stmt::Stop
            | Stmt::Clear
            | Stmt::Last { .. }
            | Stmt::Next { .. }
            | Stmt::NoPerl => {}
            Stmt::Get(e) => {
                self.eval(e, SCALAR);
            }
            Stmt::Call(e) => {
                self.eval(e, 0);
            }
            Stmt::Set {
                assigns,
                is_default,
            } => {
                for (target, value) in assigns {
                    self.assign(target, value, *is_default);
                }
            }
            Stmt::Insert { files, args } => {
                self.args(args);
                for f in files {
                    self.template_name(f);
                }
            }
            Stmt::Include { files, args } => self.include(files, args, true, None),
            Stmt::Process { files, args } => self.include(files, args, false, None),
            Stmt::Throw { types, args } => {
                for t in types {
                    self.eval(t, SCALAR);
                }
                self.args(args);
            }
            Stmt::Debug { args, .. } => self.args(args),
            Stmt::Wrapper { files, args, block } => {
                self.block(block);
                let content = self.fresh(STRING);
                self.include(files, args, true, Some(content));
            }
            Stmt::Filter {
                names,
                args,
                alias,
                block,
            } => {
                for n in names {
                    match n {
                        Expr::Str(name) => {
                            self.filters.insert(name.clone());
                            self.filter_args(name, args);
                        }
                        // `FILTER $name`: the variable holds a filter name.
                        n => {
                            self.eval(n, STRING);
                            self.args(args);
                            self.warn("filter name computed at run time");
                        }
                    }
                }
                if let Some(alias) = alias.as_ref().and_then(target_name) {
                    self.filter_aliases.insert(alias.to_string());
                }
                self.block(block);
            }
            Stmt::If {
                branches,
                else_block,
            } => self.conditional(|s| {
                for (test, body) in branches {
                    s.eval(test, 0);
                    s.block(body);
                }
                if let Some(b) = else_block {
                    s.block(b);
                }
            }),
            Stmt::Switch {
                expr,
                cases,
                default_block,
            } => self.conditional(|s| {
                s.eval(expr, SCALAR);
                for (case, body) in cases {
                    s.eval(case, 0);
                    s.block(body);
                }
                if let Some(b) = default_block {
                    s.block(b);
                }
            }),
            Stmt::Foreach {
                loop_var,
                list,
                args,
                block,
                ..
            } => {
                let list = self.eval(list, 0);
                self.args(args);
                let item = self.items(list);
                if loop_var.is_none() {
                    self.warn("FOREACH without a loop variable: names in its body may come from the items");
                }
                self.conditional(|s| {
                    let saved = s.unbind("loop");
                    let counter = s.fresh(OBJECT);
                    s.bind_node("loop", counter);
                    if let Some(var) = loop_var {
                        s.bind_node(var, item);
                    }
                    s.block(block);
                    s.restore("loop", saved);
                });
            }
            Stmt::While { expr, block, .. } => self.conditional(|s| {
                s.eval(expr, 0);
                s.block(block);
            }),
            Stmt::Try {
                block,
                catches,
                final_block,
            } => {
                self.block(block);
                self.conditional(|s| {
                    for (_, body) in catches {
                        let saved = (s.unbind("error"), s.unbind("e"));
                        let error = s.fresh(OBJECT);
                        s.bind_node("error", error);
                        s.bind_node("e", error);
                        s.block(body);
                        s.restore("error", saved.0);
                        s.restore("e", saved.1);
                    }
                });
                if let Some(b) = final_block {
                    self.block(b);
                }
            }
            Stmt::Use { names, args, alias } => {
                self.args(args);
                match names.first() {
                    Some(Expr::Str(module)) => {
                        self.modules.insert(module.clone());
                    }
                    Some(other) => {
                        self.eval(other, STRING);
                        self.warn("USE of a module named at run time");
                    }
                    None => {}
                }
                let name = alias
                    .as_ref()
                    .and_then(target_name)
                    .or(match names.first() {
                        Some(Expr::Str(s)) => Some(s.as_str()),
                        _ => None,
                    });
                if let Some(name) = name {
                    let plugin = self.fresh(OBJECT);
                    self.bind_node(name, plugin);
                }
            }
            Stmt::View {
                names,
                args,
                block,
                defblocks,
            } => {
                self.args(args);
                self.conditional(|s| {
                    s.frames.push(HashMap::new());
                    for name in ["view", "item"] {
                        let v = s.fresh(0);
                        s.bind_node(name, v);
                    }
                    s.block(block);
                    for b in defblocks.values() {
                        s.block(b);
                    }
                    s.frames.pop();
                });
                if let Some(Expr::Str(name)) = names.first() {
                    let view = self.fresh(OBJECT);
                    self.bind_node(name, view);
                }
            }
            Stmt::AnonBlock(b) => self.block(b),
            Stmt::Capture { target, block } => {
                self.block(block);
                let text = self.fresh(STRING);
                self.assign_node(target, text, false, None);
            }
            Stmt::Macro {
                ident,
                params,
                block,
            } => {
                let m = Rc::new(MacroDef {
                    params: params.clone().unwrap_or_default(),
                    block: block.clone(),
                });
                self.macros.push(m.clone());
                self.bind(ident, Binding::Macro(m));
            }
        }
    }

    fn assign(&mut self, target: &AssignTarget, value: &Expr, is_default: bool) -> NodeId {
        let v = self.eval(value, 0);
        self.assign_node(target, v, is_default, literal(value));
        v
    }

    fn assign_node(
        &mut self,
        target: &AssignTarget,
        v: NodeId,
        is_default: bool,
        lit: Option<Json>,
    ) {
        if let Some(name) = target_name(target) {
            return self.set_var(name, v, is_default, lit);
        }
        match target {
            AssignTarget::Ident(path) if !path.is_empty() => {
                // `a.b.c = v` changes `a.b`, so `c` is not required of the caller.
                let (last, parent_path) = path.split_last().unwrap();
                let parent = if parent_path.is_empty() {
                    match &last.key {
                        Expr::Str(_) => unreachable!("single names are handled by target_name"),
                        _ => {
                            self.eval(&last.key, SCALAR);
                            return;
                        }
                    }
                } else {
                    self.ident(parent_path, 0, false)
                };
                match &last.key {
                    Expr::Str(k) if !is_private(k) => {
                        let c = self.child(parent, k, false);
                        self.nodes[c].optional = true;
                    }
                    Expr::Str(_) => {}
                    key => {
                        self.eval(key, SCALAR);
                    }
                }
            }
            AssignTarget::Ident(_) => {}
            AssignTarget::Expr(e) => {
                self.eval(e, SCALAR);
            }
        }
    }

    fn set_var(&mut self, name: &str, v: NodeId, is_default: bool, lit: Option<Json>) {
        if is_default {
            // DEFAULT only assigns when the caller left the name unset.
            if self.lookup(name).is_none() {
                let id = self.optional_input(name);
                if self.nodes[id].default.is_none() {
                    self.nodes[id].default = lit;
                }
            }
            return;
        }
        self.bind_node(name, v);
    }

    fn template_name(&mut self, e: &Expr) -> Option<String> {
        match e {
            Expr::Str(s) => Some(s.clone()),
            other => {
                self.eval(other, STRING);
                self.warn("template name computed at run time: its variables are not included");
                None
            }
        }
    }

    /// INCLUDE (`localise`), PROCESS, and WRAPPER (with `content`).
    fn include(
        &mut self,
        files: &[Expr],
        args: &ArgsSpec,
        localise: bool,
        content: Option<NodeId>,
    ) {
        let mut vars = Vec::new();
        for p in &args.positional {
            self.eval(p, 0);
        }
        for named in &args.named {
            match named {
                NamedArg::KeyValue(Expr::Str(k), v) => {
                    let v = self.eval(v, 0);
                    vars.push((k.clone(), v));
                }
                NamedArg::KeyValue(k, v) => {
                    self.eval(k, SCALAR);
                    self.eval(v, 0);
                }
                NamedArg::Assign(path, v) => {
                    let target = AssignTarget::Ident(path.clone());
                    let v = self.eval(v, 0);
                    match target_name(&target) {
                        Some(k) => vars.push((k.to_string(), v)),
                        None => self.assign_node(&target, v, false, None),
                    }
                }
            }
        }
        let names: Vec<String> = files.iter().filter_map(|f| self.template_name(f)).collect();
        if localise {
            self.frames.push(HashMap::new());
        }
        for (k, v) in vars {
            self.bind_node(&k, v);
        }
        if let Some(c) = content {
            self.bind_node("content", c);
        }
        for name in names {
            self.process_template(&name, localise);
        }
        if localise {
            self.frames.pop();
        }
    }

    /// A template or BLOCK by name, looked up as TT's Context::template
    /// does: the context's BLOCKS, then those of the templates being
    /// processed, then the providers.
    fn process_template(&mut self, name: &str, localise: bool) {
        let block = self.context_blocks.get(name).cloned();
        if let Some(b) =
            block.or_else(|| self.blocks.iter().rev().find_map(|m| m.get(name).cloned()))
        {
            self.run(Self::block_key(&b), name, |s| s.block(&b));
            return;
        }
        let Some((doc, blocks)) = self.load(name) else {
            return;
        };
        if !localise {
            self.context_blocks
                .extend(blocks.iter().map(|(k, b)| (k.clone(), b.clone())));
        }
        self.run(format!("file:{name}"), name, |s| {
            let (file, line) = (
                std::mem::replace(&mut s.file, name.to_string()),
                s.line.take(),
            );
            s.blocks.push(blocks);
            s.block(&doc.block);
            s.blocks.pop();
            (s.file, s.line) = (file, line);
        });
    }

    fn load(&mut self, name: &str) -> Option<(Rc<Document>, BlockMap)> {
        if let Some(cached) = self.docs.get(name) {
            return cached.clone();
        }
        let loaded = match self.loader.as_mut() {
            None => Err("no template loader".to_string()),
            Some(load) => load(name),
        };
        let entry = match loaded {
            Ok(doc) => {
                let blocks = block_map(&doc);
                Some((doc, blocks))
            }
            Err(e) => {
                self.warn(format_args!("cannot follow '{name}': {e}"));
                None
            }
        };
        self.docs.insert(name.to_string(), entry.clone());
        entry
    }

    fn call_macro(&mut self, m: &Rc<MacroDef>, args: Option<&ArgsSpec>) -> NodeId {
        let mut frame = HashMap::new();
        if let Some(args) = args {
            for (i, p) in args.positional.iter().enumerate() {
                let v = self.eval(p, 0);
                if let Some(param) = m.params.get(i) {
                    frame.insert(
                        param.clone(),
                        Binding::Value {
                            node: v,
                            base: self.cond,
                        },
                    );
                }
            }
            // Named arguments become variables in the macro's scope.
            for named in &args.named {
                let (key, v) = match named {
                    NamedArg::KeyValue(Expr::Str(k), v) => (Some(k.clone()), v),
                    NamedArg::KeyValue(k, v) => {
                        self.eval(k, SCALAR);
                        (None, v)
                    }
                    NamedArg::Assign(path, v) => (
                        target_name(&AssignTarget::Ident(path.clone())).map(str::to_string),
                        v,
                    ),
                };
                let v = self.eval(v, 0);
                if let Some(key) = key {
                    frame.insert(
                        key,
                        Binding::Value {
                            node: v,
                            base: self.cond,
                        },
                    );
                }
            }
        }
        // Parameters the call leaves out are undefined, not inputs.
        for p in &m.params {
            if !frame.contains_key(p) {
                let v = self.fresh(0);
                frame.insert(
                    p.clone(),
                    Binding::Value {
                        node: v,
                        base: usize::MAX,
                    },
                );
            }
        }
        self.run(Self::macro_key(m), "MACRO", |s| {
            s.frames.push(frame);
            s.block(&m.block);
            s.frames.pop();
        });
        self.fresh(STRING)
    }

    /// The arguments of a built-in filter have known types.
    fn filter_args(&mut self, name: &str, args: &ArgsSpec) {
        let hints: &[u8] = match name {
            "truncate" => &[NUMBER, SCALAR],
            "repeat" => &[NUMBER],
            "indent" | "format" | "remove" | "redirect" | "file" => &[SCALAR],
            "replace" => &[SCALAR, SCALAR],
            "eval" | "evaltt" => {
                self.warn(format_args!("the '{name}' filter runs its text as a template: its variables are not included"));
                &[]
            }
            _ => &[],
        };
        for (i, p) in args.positional.iter().enumerate() {
            self.eval(p, hints.get(i).copied().unwrap_or(0));
        }
        self.args(&ArgsSpec {
            positional: Vec::new(),
            named: args.named.clone(),
        });
    }

    fn args(&mut self, args: &ArgsSpec) {
        for p in &args.positional {
            self.eval(p, 0);
        }
        for named in &args.named {
            match named {
                NamedArg::KeyValue(k, v) => {
                    if !matches!(k, Expr::Str(_)) {
                        self.eval(k, SCALAR);
                    }
                    self.eval(v, 0);
                }
                NamedArg::Assign(_, v) => {
                    self.eval(v, 0);
                }
            }
        }
    }

    // --- expressions ---

    /// Scan `e`, used as a value of the `hint` types (0 for any), and
    /// return the node describing its value.
    fn eval(&mut self, e: &Expr, hint: u8) -> NodeId {
        match e {
            Expr::Str(_) => self.fresh(STRING),
            Expr::Int(_) | Expr::Float(_) => self.fresh(NUMBER),
            Expr::Ident(path) => self.ident(path, hint, true),
            Expr::IdentRef(path) => {
                self.ident(path, 0, false);
                self.fresh(0)
            }
            Expr::Quoted(parts) => {
                for p in parts {
                    self.eval(p, SCALAR);
                }
                self.fresh(STRING)
            }
            Expr::List(items) => {
                for i in items {
                    self.eval(i, 0);
                }
                self.fresh(ARRAY)
            }
            Expr::Range(from, to) => {
                self.eval(from, NUMBER);
                self.eval(to, NUMBER);
                self.list_of(NUMBER)
            }
            Expr::Hash(pairs) => {
                let hash = self.fresh(OBJECT);
                for (k, v) in pairs {
                    let v = self.eval(v, 0);
                    match k {
                        Expr::Str(k) => {
                            self.nodes[hash].props.insert(k.clone(), v);
                        }
                        k => {
                            self.eval(k, SCALAR);
                        }
                    }
                }
                hash
            }
            Expr::BinOp { op, left, right } => match op {
                BinOp::And | BinOp::Or => {
                    let l = self.conditional(|s| s.eval(left, 0));
                    self.conditional(|s| s.eval(right, 0));
                    if *op == BinOp::Or
                        && matches!(**left, Expr::Ident(_))
                        && self.nodes[l].default.is_none()
                    {
                        self.nodes[l].default = literal(right);
                    }
                    self.fresh(0)
                }
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::IntDiv | BinOp::Mod => {
                    self.eval(left, NUMBER);
                    self.eval(right, NUMBER);
                    self.fresh(NUMBER)
                }
                BinOp::Cat => {
                    self.eval(left, SCALAR);
                    self.eval(right, SCALAR);
                    self.fresh(STRING)
                }
                BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                    self.eval(left, SCALAR);
                    self.eval(right, SCALAR);
                    self.fresh(0)
                }
            },
            Expr::Not(e) => {
                self.conditional(|s| s.eval(e, 0));
                self.fresh(0)
            }
            Expr::Paren(e) => self.eval(e, hint),
            Expr::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                self.conditional(|s| {
                    s.eval(cond, 0);
                    s.eval(then_expr, hint);
                    s.eval(else_expr, hint);
                });
                self.fresh(0)
            }
            Expr::Assign { target, expr } => self.assign(target, expr, false),
        }
    }

    /// A dotted variable.  `require` is false where the use does not need
    /// the value to exist (`\ref`, the parent of an assignment).
    fn ident(&mut self, path: &[IdentNode], hint: u8, require: bool) -> NodeId {
        let Some((first, rest)) = path.split_first() else {
            return self.fresh(0);
        };
        // `base` is the depth at which uses of this name are unconditional.
        let (mut cur, base) = match &first.key {
            Expr::Str(name) => match self.lookup(name) {
                Some(Binding::Macro(m)) => {
                    let out = self.call_macro(&m, first.args.as_ref());
                    (out, usize::MAX)
                }
                Some(Binding::Value { node, base }) if base <= self.cond => (node, base),
                // Set conditionally and read outside that conditional: it
                // may also come from the caller.
                Some(Binding::Value { .. }) => (self.optional_input(name), 0),
                None if is_private(name) || BUILTINS.contains(&name.as_str()) => {
                    (self.fresh(0), usize::MAX)
                }
                None => (self.child(0, name, require && self.cond == 0), 0),
            },
            key => {
                self.eval(key, SCALAR);
                (self.fresh(0), usize::MAX)
            }
        };
        let is_macro =
            matches!(&first.key, Expr::Str(n) if matches!(self.lookup(n), Some(Binding::Macro(_))));
        if let (Some(args), false) = (&first.args, is_macro) {
            self.args(args);
            self.nodes[cur].callable = true;
            cur = self.fresh(0);
        }
        let required = require && self.cond == base;
        for seg in rest {
            cur = self.step(cur, seg, required);
        }
        self.hint(cur, hint);
        cur
    }

    fn step(&mut self, parent: NodeId, seg: &IdentNode, required: bool) -> NodeId {
        match &seg.key {
            Expr::Int(_) => {
                if let Some(a) = &seg.args {
                    self.args(a);
                }
                self.items(parent)
            }
            Expr::Str(k) if k.parse::<u64>().is_ok() => self.items(parent),
            Expr::Str(k) if is_private(k) => self.fresh(0),
            Expr::Str(k) => {
                if let Some(out) = self.vmethod(parent, k, seg.args.as_ref(), required) {
                    return out;
                }
                let c = self.child(parent, k, required);
                match &seg.args {
                    Some(a) => {
                        self.args(a);
                        self.nodes[c].callable = true;
                        self.fresh(0)
                    }
                    None => c,
                }
            }
            key => {
                self.eval(key, SCALAR);
                if let Some(a) = &seg.args {
                    self.args(a);
                }
                self.any_key(parent)
            }
        }
    }

    /// Model the built-in virtual method `name` on `target`, or None if
    /// `name` is not one.  `required` is as for a key at this step.
    fn vmethod(
        &mut self,
        target: NodeId,
        name: &str,
        args: Option<&ArgsSpec>,
        required: bool,
    ) -> Option<NodeId> {
        let known = matches!(
            name,
            "first"
                | "last"
                | "pop"
                | "shift"
                | "max"
                | "sort"
                | "nsort"
                | "reverse"
                | "unique"
                | "grep"
                | "slice"
                | "splice"
                | "merge"
                | "list"
                | "join"
                | "push"
                | "unshift"
                | "keys"
                | "values"
                | "pairs"
                | "each"
                | "items"
                | "exists"
                | "delete"
                | "import"
                | "size"
                | "length"
                | "defined"
                | "empty"
                | "hash"
                | "item"
                | "upper"
                | "lower"
                | "ucfirst"
                | "lcfirst"
                | "trim"
                | "collapse"
                | "squote"
                | "dquote"
                | "html"
                | "xml"
                | "repeat"
                | "replace"
                | "remove"
                | "substr"
                | "match"
                | "split"
                | "chunk"
                | "search"
                | "inc"
                | "dec"
        );
        // Common field names that are also vmethods only count as vmethods
        // when called with parentheses.
        let ambiguous = matches!(name, "items" | "item" | "hash" | "list");
        if !known || (ambiguous && args.is_none()) {
            return None;
        }
        // `item` with a literal key is that key: var.item('name') is
        // var.name, and var.item(0) is var.0.
        if let (Some(a), "item") = (args, name) {
            if let ([key], []) = (a.positional.as_slice(), a.named.as_slice()) {
                match key {
                    Expr::Str(k) if k.parse::<u64>().is_ok() => return Some(self.items(target)),
                    Expr::Str(k) if is_private(k) => return Some(self.fresh(0)),
                    Expr::Str(k) => return Some(self.child(target, k, required)),
                    Expr::Int(_) => return Some(self.items(target)),
                    _ => {}
                }
            }
        }
        if let Some(a) = args {
            self.args(a);
        }
        let out = match name {
            "first" | "last" | "pop" | "shift" | "max" => self.items(target),
            "reverse" | "unique" | "grep" | "slice" | "splice" | "merge" => {
                self.hint(target, ARRAY);
                target
            }
            "sort" | "nsort" => {
                self.hint(target, ARRAY | OBJECT);
                target
            }
            "list" => target,
            "join" => {
                self.hint(target, ARRAY);
                self.fresh(STRING)
            }
            "push" | "unshift" => {
                self.hint(target, ARRAY);
                self.fresh(0)
            }
            "keys" => {
                self.hint(target, OBJECT);
                self.list_of(STRING)
            }
            "values" => {
                self.hint(target, OBJECT);
                let value = self.any_key(target);
                let list = self.fresh(ARRAY);
                self.nodes[list].items = Some(value);
                list
            }
            "pairs" => {
                self.hint(target, OBJECT);
                let value = self.any_key(target);
                let pair = self.fresh(OBJECT);
                let key = self.fresh(STRING);
                self.nodes[pair].props.insert("key".into(), key);
                self.nodes[pair].props.insert("value".into(), value);
                let list = self.fresh(ARRAY);
                self.nodes[list].items = Some(pair);
                list
            }
            "each" | "items" => {
                self.hint(target, OBJECT);
                self.fresh(ARRAY)
            }
            "exists" | "delete" | "import" => {
                self.hint(target, OBJECT);
                self.fresh(0)
            }
            "size" => {
                self.hint(target, ARRAY | OBJECT);
                self.fresh(NUMBER)
            }
            "length" => {
                self.hint(target, SCALAR);
                self.fresh(NUMBER)
            }
            "match" | "split" | "chunk" => {
                self.hint(target, SCALAR);
                self.list_of(STRING)
            }
            "search" => {
                self.hint(target, SCALAR);
                self.fresh(0)
            }
            "inc" | "dec" => {
                self.hint(target, NUMBER);
                self.fresh(NUMBER)
            }
            "defined" | "empty" | "hash" | "item" => self.fresh(0),
            _ => {
                // The text vmethods: upper, lower, trim, replace, ...
                self.hint(target, SCALAR);
                self.fresh(STRING)
            }
        };
        Some(out)
    }

    // --- output ---

    fn emit(&self, id: NodeId, depth: usize) -> Json {
        let n = &self.nodes[id];
        let mut out = Map::new();
        if depth > 64 {
            return Json::Object(out);
        }
        let kinds = match (n.union, n.inter) {
            (0, _) => 0,
            (union, 0) => union,
            (_, inter) => inter,
        };
        if n.callable {
            out.insert("x-tt-callable".into(), json!(true));
        } else if kinds != 0 && kinds != ANY {
            let names: Vec<&str> = [
                (OBJECT, "object"),
                (ARRAY, "array"),
                (STRING, "string"),
                (NUMBER, "number"),
            ]
            .into_iter()
            .filter(|(k, _)| kinds & k != 0)
            .map(|(_, name)| name)
            .collect();
            out.insert(
                "type".into(),
                if names.len() == 1 {
                    json!(names[0])
                } else {
                    json!(names)
                },
            );
        }
        if kinds & OBJECT != 0 && !n.props.is_empty() {
            let props: Map<String, Json> = n
                .props
                .iter()
                .map(|(k, &c)| (k.clone(), self.emit(c, depth + 1)))
                .collect();
            out.insert("properties".into(), Json::Object(props));
            let required: Vec<&String> = n
                .required
                .iter()
                .filter(|k| {
                    let c = &self.nodes[n.props[*k]];
                    !c.optional && !c.callable
                })
                .collect();
            if !required.is_empty() {
                out.insert("required".into(), json!(required));
            }
        }
        if kinds & OBJECT != 0 && kinds & ARRAY == 0 {
            if let Some(v) = n.any_key {
                out.insert("additionalProperties".into(), self.emit(v, depth + 1));
            }
        }
        if kinds & ARRAY != 0 {
            if let Some(item) = n.items.or(n.any_key) {
                out.insert("items".into(), self.emit(item, depth + 1));
            }
        }
        if let Some(d) = &n.default {
            out.insert("default".into(), d.clone());
        }
        if !n.sources.is_empty() {
            out.insert("x-tt-source".into(), json!(n.sources));
        }
        Json::Object(out)
    }
}
