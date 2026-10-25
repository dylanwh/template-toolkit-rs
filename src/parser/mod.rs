pub mod ast;
pub mod grammar_tables;
pub mod lexer;

use std::{collections::HashMap, rc::Rc};

use crate::{
    constants::{CHOMP_NONE, DEBUG_DIRS, DEBUG_FLAGS, DEBUG_PARSER},
    parser::{
        ast::{
            ArgsSpec, AssignTarget, BinOp, Block, Expr, IdentNode, NameArgs, NamedArg,
            ParsedDocument, SpannedStmt, Stmt, VariableTrace,
        },
        grammar_tables::{RULES, STATES},
        lexer::{
            get_tag_style, interpolate_text, split_text, tokenise_directive, ParserStyle,
            SplitChunk,
        },
    },
    stash::Stash,
    value::Value,
};

/// Compile-time constant folding namespace (`Template::Namespace::Constants`).
#[derive(Debug, Clone)]
pub struct ConstantsNamespace {
    pub stash: Stash,
}

impl ConstantsNamespace {
    pub fn new(constants: HashMap<String, Value>) -> Self {
        Self {
            stash: Stash::new(Some(constants)),
        }
    }

    pub fn from_stash(stash: Stash) -> Self {
        Self { stash }
    }

    /// Attempt to fold an `Ident` (`Vec<IdentNode>`) at compile time.
    /// Returns `Some(String)` if folded to a constant scalar string, or `None` if deferred to runtime.
    pub fn fold_ident(&self, nodes: &[IdentNode]) -> Option<String> {
        if nodes.len() < 2 {
            return None;
        }
        // Discard first node indicating constants namespace
        let rest = &nodes[1..];
        let mut pairs = Vec::with_capacity(rest.len());

        for node in rest {
            let key_str = match &node.key {
                Expr::Str(s) => s.clone(),
                Expr::Int(i) => i.to_string(),
                Expr::Float(f) => crate::value::format_perl_float(*f),
                _ => return None, // non-constant key -> defer
            };
            let mut arg_vals = Vec::new();
            if let Some(args_spec) = &node.args {
                for pos_expr in &args_spec.positional {
                    arg_vals.push(Self::eval_const_expr(pos_expr)?);
                }
                if !args_spec.named.is_empty() {
                    let mut map = HashMap::new();
                    for narg in &args_spec.named {
                        match narg {
                            NamedArg::KeyValue(k, v) => {
                                let ks = Self::eval_const_expr(k)?.to_tt_string();
                                let vv = Self::eval_const_expr(v)?;
                                map.insert(ks, vv);
                            }
                            NamedArg::Assign(_, _) => return None,
                        }
                    }
                    arg_vals.push(Value::from_hash(map));
                }
            }
            pairs.push((Value::Str(key_str), arg_vals));
        }

        match self.stash.get_pairs(&pairs, None) {
            Ok(val) => {
                if !val.is_defined() || val.is_ref() {
                    None
                } else {
                    let s = val.to_tt_string();
                    if s.is_empty() {
                        None
                    } else {
                        Some(s)
                    }
                }
            }
            Err(_) => None,
        }
    }

    fn eval_const_expr(expr: &Expr) -> Option<Value> {
        match expr {
            Expr::Str(s) => Some(Value::Str(s.clone())),
            Expr::Int(i) => Some(Value::Int(*i)),
            Expr::Float(f) => Some(Value::Float(*f)),
            Expr::List(items) => {
                let mut v = Vec::with_capacity(items.len());
                for item in items {
                    v.push(Self::eval_const_expr(item)?);
                }
                Some(Value::from_list(v))
            }
            Expr::Hash(entries) => {
                let mut m = HashMap::new();
                for (k, val) in entries {
                    let ks = Self::eval_const_expr(k)?.to_tt_string();
                    let vv = Self::eval_const_expr(val)?;
                    m.insert(ks, vv);
                }
                Some(Value::from_hash(m))
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ParseInfo {
    pub name: Option<String>,
    pub path: Option<String>,
    pub debug: Option<bool>,
}

/// Callback receiving parser warnings (e.g. duplicate BLOCK definitions).
pub type WarnHandler = Rc<dyn Fn(&str)>;

#[derive(Clone)]
pub struct Parser {
    pub styles: Vec<ParserStyle>,
    pub debug: u32,
    pub debug_dirs: bool,
    pub file_info: bool,
    pub trace_vars: bool,
    pub namespaces: HashMap<String, ConstantsNamespace>,
    pub error_msg: String,
    pub warnings: Vec<String>,
    pub warn_handler: Option<WarnHandler>,
}

impl std::fmt::Debug for Parser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Parser").finish_non_exhaustive()
    }
}

#[bon::bon]
impl Parser {
    #[builder(start_fn = builder, finish_fn = build)]
    pub fn build(
        start_tag: Option<String>,
        end_tag: Option<String>,
        outline_tag: Option<String>,
        tag_style: Option<String>,
        #[builder(default = false)] anycase: bool,
        #[builder(default = false)] interpolate: bool,
        #[builder(default = CHOMP_NONE)] pre_chomp: usize,
        #[builder(default = CHOMP_NONE)] post_chomp: usize,
        #[builder(default = false)] v1dollar: bool,
        #[builder(default = false)] eval_perl: bool,
        #[builder(default = true)] file_info: bool,
        #[builder(default = false)] trace_vars: bool,
        #[builder(default = 0)] debug: u32,
        #[builder(default)] namespaces: HashMap<String, ConstantsNamespace>,
        warn_handler: Option<WarnHandler>,
    ) -> Result<Self, String> {
        let mut parser = Self {
            styles: Vec::new(),
            debug: debug & (DEBUG_PARSER | DEBUG_FLAGS),
            debug_dirs: (debug & DEBUG_DIRS) != 0,
            file_info,
            trace_vars,
            namespaces,
            error_msg: String::new(),
            warnings: Vec::new(),
            warn_handler,
        };

        let cfg = StyleConfig {
            tag_style: tag_style.or_else(|| Some("default".to_string())),
            start_tag,
            end_tag,
            outline_tag,
            anycase: Some(anycase),
            interpolate: Some(interpolate),
            pre_chomp: Some(pre_chomp),
            post_chomp: Some(post_chomp),
            v1dollar: Some(v1dollar),
            eval_perl: Some(eval_perl),
        };
        if cfg.start_tag.is_some() || cfg.end_tag.is_some() {
            // Explicit tags override default tag_style unless specified
        }
        parser.new_style(cfg)?;
        Ok(parser)
    }
}

#[derive(Debug, Clone, Default)]
pub struct StyleConfig {
    pub tag_style: Option<String>,
    pub start_tag: Option<String>,
    pub end_tag: Option<String>,
    pub outline_tag: Option<String>,
    pub anycase: Option<bool>,
    pub interpolate: Option<bool>,
    pub pre_chomp: Option<usize>,
    pub post_chomp: Option<usize>,
    pub v1dollar: Option<bool>,
    pub eval_perl: Option<bool>,
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser {
    pub fn new() -> Self {
        Self::builder().build().unwrap()
    }

    pub fn error(&self) -> &str {
        &self.error_msg
    }

    pub fn current_style(&self) -> &ParserStyle {
        self.styles.last().expect("at least one style")
    }

    pub fn new_style(&mut self, mut config: StyleConfig) -> Result<&ParserStyle, String> {
        let mut style = self.styles.last().cloned().unwrap_or_default();

        if let Some(ref ts) = config.tag_style {
            let Some((s, e, o)) = get_tag_style(ts) else {
                self.error_msg = format!("Invalid tag style: {}", ts);
                return Err(self.error_msg.clone());
            };
            if config.start_tag.is_none() {
                config.start_tag = Some(s.to_string());
            }
            if config.end_tag.is_none() {
                config.end_tag = Some(e.to_string());
            }
            if config.outline_tag.is_none() && o.is_some() {
                config.outline_tag = o.map(|x| x.to_string());
            }
        }

        if let Some(s) = config.start_tag {
            style.start_tag = s;
        }
        if let Some(e) = config.end_tag {
            style.end_tag = e;
        }
        if let Some(o) = config.outline_tag {
            style.outline_tag = Some(o);
        }
        if let Some(v) = config.anycase {
            style.anycase = v;
        }
        if let Some(v) = config.interpolate {
            style.interpolate = v;
        }
        if let Some(v) = config.pre_chomp {
            style.pre_chomp = v;
        }
        if let Some(v) = config.post_chomp {
            style.post_chomp = v;
        }
        if let Some(v) = config.v1dollar {
            style.v1dollar = v;
        }
        if let Some(v) = config.eval_perl {
            style.eval_perl = v;
        }

        self.styles.push(style);
        Ok(self.styles.last().unwrap())
    }

    pub fn old_style(&mut self) -> Result<&ParserStyle, String> {
        if self.styles.len() <= 1 {
            self.error_msg = "only 1 parser style remaining".to_string();
            return Err(self.error_msg.clone());
        }
        self.styles.pop();
        Ok(self.styles.last().unwrap())
    }

    pub fn split_text(&self, text: &str) -> Result<Vec<SplitChunk>, String> {
        split_text(text, self.current_style())
    }

    pub fn interpolate_text(&self, text: &str, line: usize) -> Vec<SplitChunk> {
        interpolate_text(text, line, self.current_style().anycase)
    }

    pub fn tokenise_directive(&self, text: &str) -> Vec<(String, String)> {
        tokenise_directive(text, self.current_style().anycase)
    }

    pub fn parse(&mut self, text: &str, info: Option<ParseInfo>) -> Result<ParsedDocument, String> {
        self.error_msg.clear();
        let info = info.unwrap_or_default();
        let chunks = match self.split_text(text) {
            Ok(c) => c,
            Err(e) => {
                self.error_msg = e.clone();
                return Err(e);
            }
        };

        let mut state = ParseSession {
            parser: self,
            info,
            in_block: Vec::new(),
            defblock: HashMap::new(),
            defblock_stack: Vec::new(),
            defblocks_name_stack: Vec::new(),
            metadata: Vec::new(),
            variables: HashMap::new(),
            line: String::new(),
            inperl: 0,
            debug_dirs: false,
            wrap_debug: false,
        };
        state.debug_dirs = state.info.debug.unwrap_or(state.parser.debug_dirs);
        state.wrap_debug = state.debug_dirs;

        let block = state.run_parse(chunks)?;
        let mut meta_map = HashMap::new();
        for (k, v) in state.metadata {
            meta_map.insert(k, v);
        }

        Ok(ParsedDocument {
            block,
            defblocks: state.defblock,
            metadata: meta_map,
            variables: state.variables,
        })
    }
}

#[derive(Debug, Clone)]
enum StackVal {
    None,
    Token(String),
    Block(Block),
    Chunks(Vec<SpannedStmt>),
    Chunk(Option<SpannedStmt>),
    Stmt(Option<Stmt>),
    Else {
        elsifs: Vec<(Expr, Block)>,
        else_block: Option<Block>,
    },
    Case {
        cases: Vec<(Expr, Block)>,
        default_block: Option<Block>,
    },
    LoopVar {
        var: Option<String>,
        list: Expr,
        args: ArgsSpec,
    },
    Final {
        catches: Vec<(Option<String>, Block)>,
        final_block: Option<Block>,
    },
    MetaList(Vec<(String, String)>),
    StrList(Vec<String>),
    Expr(Expr),
    AssignTarget(AssignTarget),
    ExprList(Vec<Expr>),
    OptExpr(Option<Expr>),
    Range(Expr, Expr),
    Params(Vec<(Expr, Expr)>),
    Ident(Vec<IdentNode>),
    SetList(Vec<(AssignTarget, Expr)>),
    Args(ArgsSpec),
    NameArgs(NameArgs),
}

impl StackVal {
    fn into_token(self) -> String {
        match self {
            StackVal::Token(s) => s,
            StackVal::Expr(Expr::Str(s)) => s,
            _ => String::new(),
        }
    }

    fn into_block(self) -> Block {
        match self {
            StackVal::Block(b) => b,
            StackVal::Stmt(Some(s)) => Block {
                stmts: vec![SpannedStmt {
                    line: None,
                    file: None,
                    stmt: s,
                }],
            },
            _ => Block::default(),
        }
    }

    fn into_expr(self) -> Expr {
        match self {
            StackVal::Expr(e) => e,
            StackVal::Token(t) => Expr::Str(unquote_literal(&t)),
            _ => Expr::Str(String::new()),
        }
    }

    fn into_stmt(self) -> Option<Stmt> {
        match self {
            StackVal::Stmt(s) => s,
            _ => None,
        }
    }

    fn into_ident(self) -> Vec<IdentNode> {
        match self {
            StackVal::Ident(v) => v,
            _ => Vec::new(),
        }
    }

    fn into_args(self) -> ArgsSpec {
        match self {
            StackVal::Args(a) => a,
            _ => ArgsSpec::default(),
        }
    }

    fn into_nameargs(self) -> NameArgs {
        match self {
            StackVal::NameArgs(na) => na,
            _ => NameArgs {
                names: Vec::new(),
                args: ArgsSpec::default(),
                alias: None,
            },
        }
    }
}

// TT compiles `expr OP expr` by splicing the operand source text together and
// letting Perl re-parse it, so Perl's operator precedence (not the grammar's)
// decides evaluation order. Emulate that by flattening and re-parsing.
enum PTok {
    Atom(Expr),
    Op(BinOp),
    Not,
}

fn perl_flatten(e: Expr, out: &mut Vec<PTok>) {
    match e {
        Expr::BinOp { op, left, right } if op != BinOp::IntDiv => {
            perl_flatten(*left, out);
            out.push(PTok::Op(op));
            perl_flatten(*right, out);
        }
        Expr::Not(x) => {
            out.push(PTok::Not);
            perl_flatten(*x, out);
        }
        other => out.push(PTok::Atom(other)),
    }
}

fn perl_prec(op: BinOp) -> u8 {
    match op {
        BinOp::Mul | BinOp::Div | BinOp::Mod => 7,
        BinOp::Add | BinOp::Sub | BinOp::Cat => 6,
        BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => 5,
        BinOp::Eq | BinOp::Ne => 4,
        BinOp::And => 3,
        BinOp::Or => 2,
        BinOp::IntDiv => 8,
    }
}

fn perl_unary(toks: &mut std::iter::Peekable<std::vec::IntoIter<PTok>>) -> Expr {
    match toks.next() {
        Some(PTok::Not) => Expr::Not(Box::new(perl_unary(toks))),
        Some(PTok::Atom(e)) => e,
        _ => Expr::Str(String::new()),
    }
}

fn perl_climb(toks: &mut std::iter::Peekable<std::vec::IntoIter<PTok>>, min: u8) -> Expr {
    let mut left = perl_unary(toks);
    while let Some(PTok::Op(op)) = toks.peek() {
        let op = *op;
        let pr = perl_prec(op);
        if pr < min {
            break;
        }
        toks.next();
        let right = perl_climb(toks, pr + 1);
        left = Expr::BinOp {
            op,
            left: Box::new(left),
            right: Box::new(right),
        };
    }
    left
}

fn perl_reparse(e: Expr) -> Expr {
    let mut toks = Vec::new();
    perl_flatten(e, &mut toks);
    perl_climb(&mut toks.into_iter().peekable(), 0)
}

fn perl_binop(op: BinOp, left: Expr, right: Expr) -> Expr {
    let e = Expr::BinOp {
        op,
        left: Box::new(left),
        right: Box::new(right),
    };
    if op == BinOp::IntDiv {
        return e;
    }
    perl_reparse(e)
}

fn perl_not(e: Expr) -> Expr {
    perl_reparse(Expr::Not(Box::new(e)))
}

fn unquote_literal(s: &str) -> String {
    let inner = if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2 {
        &s[1..s.len() - 1]
    } else {
        s
    };
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next_c) = chars.peek() {
                if next_c == '\'' || next_c == '\\' {
                    out.push(next_c);
                    chars.next();
                    continue;
                }
            }
        }
        out.push(c);
    }
    out
}

fn parse_number_token(s: &str) -> Expr {
    if s.contains('.') {
        if let Ok(f) = s.parse::<f64>() {
            return Expr::Float(f);
        }
    }
    if let Ok(i) = s.parse::<i64>() {
        Expr::Int(i)
    } else if let Ok(f) = s.parse::<f64>() {
        Expr::Float(f)
    } else {
        Expr::Str(s.to_string())
    }
}

struct ParseSession<'a> {
    parser: &'a mut Parser,
    info: ParseInfo,
    in_block: Vec<String>,
    defblock: HashMap<String, Block>,
    defblock_stack: Vec<HashMap<String, Block>>,
    defblocks_name_stack: Vec<String>,
    metadata: Vec<(String, String)>,
    variables: HashMap<String, VariableTrace>,
    line: String,
    inperl: usize,
    debug_dirs: bool,
    /// $info->{DEBUG}: fixed at the start of a parse (DEBUG on/off only affects later parses)
    wrap_debug: bool,
}

impl<'a> ParseSession<'a> {
    fn enter_block(&mut self, name: &str) {
        self.in_block.push(name.to_string());
    }

    fn leave_block(&mut self) -> Option<String> {
        let label = self.block_label();
        self.in_block.pop();
        label
    }

    fn in_block_top(&self, name: &str) -> bool {
        self.in_block.last().map(|s| s == name).unwrap_or(false)
    }

    fn block_label(&self) -> Option<String> {
        self.in_block
            .last()
            .map(|name| format!("{}{}", name, self.in_block.len()))
    }

    fn push_defblock(&mut self) {
        let cur = std::mem::take(&mut self.defblock);
        self.defblock_stack.push(cur);
    }

    fn pop_defblock(&mut self) -> HashMap<String, Block> {
        let defs = std::mem::take(&mut self.defblock);
        if let Some(prev) = self.defblock_stack.pop() {
            self.defblock = prev;
        }
        defs
    }

    fn define_block(&mut self, name: String, block: Block) {
        if self.defblock.contains_key(&name) {
            let msg = format!("Block redefined: {}\n", name);
            self.parser.warnings.push(msg.clone());
            if let Some(ref h) = self.parser.warn_handler {
                h(&msg);
            }
        }
        self.defblock.insert(name, block);
    }

    fn trace_ident(&mut self, nodes: &[IdentNode]) {
        if !self.parser.trace_vars {
            return;
        }
        let mut current_map = &mut self.variables;
        for node in nodes {
            let key_str = match &node.key {
                Expr::Str(s) => s.clone(),
                Expr::Int(i) => i.to_string(),
                Expr::Float(f) => crate::value::format_perl_float(*f),
                _ => String::new(),
            };
            let entry = current_map.entry(key_str).or_default();
            current_map = &mut entry.children;
        }
    }

    fn make_ident_expr(&mut self, nodes: Vec<IdentNode>) -> Expr {
        if nodes.is_empty() {
            return Expr::Str(String::new());
        }
        self.trace_ident(&nodes);

        if nodes.len() >= 2 && nodes[0].args.is_none() {
            if let Expr::Str(ref ns_key) = nodes[0].key {
                if let Some(ns) = self.parser.namespaces.get(ns_key) {
                    if let Some(folded) = ns.fold_ident(&nodes) {
                        return Expr::Str(folded);
                    }
                }
            }
        }

        Expr::Ident(nodes)
    }

    fn parse_error(&mut self, msg: &str, dir_text: Option<&str>) -> String {
        let line_str = if self.line.is_empty() || self.line == "0" {
            "unknown".to_string()
        } else {
            self.line.clone()
        };
        let mut full = format!("line {}: {}", line_str, msg);
        if let Some(dt) = dir_text {
            full.push_str(&format!("\n  [% {} %]", dt));
        }
        self.parser.error_msg = full.clone();
        full
    }

    fn run_parse(&mut self, mut queue: Vec<SplitChunk>) -> Result<Block, String> {
        queue.reverse(); // Use as stack (pop from back = shift from front)

        let mut stack: Vec<(usize, StackVal)> = vec![(0, StackVal::None)];
        let mut token: Option<String> = None;
        let mut value: Option<String> = None;
        let mut last_dir_text: Option<String> = None;
        let mut in_string = false;
        let anycase = self.parser.current_style().anycase;

        loop {
            let stateno = stack.last().unwrap().0;
            let state = &STATES[stateno];

            let action: Option<i16> = if let Some(actions) = state.actions {
                while token.is_none() && !queue.is_empty() {
                    let item = queue.pop().unwrap();
                    match item {
                        SplitChunk::Text(t) => {
                            token = Some("TEXT".to_string());
                            value = Some(t);
                        }
                        SplitChunk::Token(tok_type, tok_val) => {
                            if tok_type == "\"" {
                                in_string = !in_string;
                            }
                            token = Some(tok_type);
                            value = Some(tok_val);
                        }
                        SplitChunk::IText { text, line } => {
                            self.line = line.to_string();
                            if self.inperl > 0 {
                                token = Some("TEXT".to_string());
                                value = Some(text);
                            } else {
                                let sub = interpolate_text(&text, line, anycase);
                                for sc in sub.into_iter().rev() {
                                    queue.push(sc);
                                }
                            }
                        }
                        SplitChunk::Directive {
                            text,
                            line,
                            tokens: dir_tokens,
                        } => {
                            last_dir_text = Some(text.clone());
                            self.line = line.clone();

                            // Push trailing ';'
                            queue.push(SplitChunk::Token(";".to_string(), ";".to_string()));
                            for (tt, tv) in dir_tokens.into_iter().rev() {
                                queue.push(SplitChunk::Token(tt, tv));
                            }

                            if self.wrap_debug && !in_string {
                                let file_name = self.info.name.clone().unwrap_or_default();
                                let debug_tokens = vec![
                                    ("DEBUG".to_string(), "DEBUG".to_string()),
                                    ("IDENT".to_string(), "msg".to_string()),
                                    ("IDENT".to_string(), "line".to_string()),
                                    ("ASSIGN".to_string(), "=".to_string()),
                                    ("LITERAL".to_string(), format!("'{}'", line)),
                                    ("IDENT".to_string(), "text".to_string()),
                                    ("ASSIGN".to_string(), "=".to_string()),
                                    ("LITERAL".to_string(), format!("'{}'", text)),
                                    ("IDENT".to_string(), "file".to_string()),
                                    ("ASSIGN".to_string(), "=".to_string()),
                                    ("LITERAL".to_string(), format!("'{}'", file_name)),
                                    (";".to_string(), ";".to_string()),
                                ];
                                for (dt, dv) in debug_tokens.into_iter().rev() {
                                    queue.push(SplitChunk::Token(dt, dv));
                                }
                            }
                        }
                    }
                }

                let lookahead = token.as_deref().unwrap_or("");
                let found = actions
                    .iter()
                    .find(|&&(k, _)| k == lookahead)
                    .map(|&(_, act)| act);
                found.or(state.default)
            } else {
                state.default
            };

            let Some(act) = action else {
                break;
            };

            if act > 0 {
                // Shift
                let shifted_val = StackVal::Token(value.take().unwrap_or_default());
                token = None;
                stack.push((act as usize, shifted_val));
                continue;
            }

            // Reduce
            let rule_idx = (-act) as usize;
            let rule = &RULES[rule_idx];
            let accept = act == 0;

            let split_idx = stack.len() - rule.len;
            let popped: Vec<StackVal> = stack.drain(split_idx..).map(|(_, v)| v).collect();

            let coderet = match self.execute_rule(rule_idx, popped) {
                Ok(v) => v,
                Err(e) => return Err(self.parse_error(&e, None)),
            };

            if accept {
                return Ok(coderet.into_block());
            }

            let top_state = stack.last().unwrap().0;
            let goto_state = STATES[top_state]
                .gotos
                .iter()
                .find(|&&(lhs, _)| lhs == rule.lhs)
                .map(|&(_, st)| st)
                .expect("valid goto state");

            stack.push((goto_state, coderet));
        }

        let Some(val) = value else {
            return Err(self.parse_error("unexpected end of input", None));
        };
        if val == ";" {
            return Err(self.parse_error("unexpected end of directive", last_dir_text.as_deref()));
        }
        Err(self.parse_error(
            &format!("unexpected token ({})", val),
            last_dir_text.as_deref(),
        ))
    }

    fn execute_rule(&mut self, rule_idx: usize, mut v: Vec<StackVal>) -> Result<StackVal, String> {
        match rule_idx {
            // Rule 0: $start -> template $end
            0 => Ok(v.into_iter().next().unwrap_or(StackVal::None)),

            // Rule 1: template -> block
            1 => Ok(v.pop().unwrap()),

            // Rule 2: block -> chunks
            2 => {
                let StackVal::Chunks(stmts) = v.pop().unwrap() else {
                    return Ok(StackVal::Block(Block::default()));
                };
                Ok(StackVal::Block(Block { stmts }))
            }

            // Rule 3: block -> /* NULL */
            3 => Ok(StackVal::Block(Block::default())),

            // Rule 4: chunks -> chunks chunk
            4 => {
                let chunk = v.pop().unwrap();
                let StackVal::Chunks(mut list) = v.pop().unwrap() else {
                    return Ok(StackVal::Chunks(Vec::new()));
                };
                if let StackVal::Chunk(Some(sp)) = chunk {
                    list.push(sp);
                }
                Ok(StackVal::Chunks(list))
            }

            // Rule 5: chunks -> chunk
            5 => {
                let chunk = v.pop().unwrap();
                let mut list = Vec::new();
                if let StackVal::Chunk(Some(sp)) = chunk {
                    list.push(sp);
                }
                Ok(StackVal::Chunks(list))
            }

            // Rule 6: chunk -> TEXT
            6 => {
                let text = v.pop().unwrap().into_token();
                Ok(StackVal::Chunk(Some(SpannedStmt {
                    line: None,
                    file: None,
                    stmt: Stmt::Text(text),
                })))
            }

            // Rule 7: chunk -> statement ';'
            7 => {
                let _semi = v.pop();
                let stmt_opt = v.pop().unwrap().into_stmt();
                if let Some(stmt) = stmt_opt {
                    let line = if self.parser.file_info && !self.line.is_empty() {
                        Some(self.line.clone())
                    } else {
                        None
                    };
                    let file = if self.parser.file_info {
                        self.info.path.clone().or_else(|| self.info.name.clone())
                    } else {
                        None
                    };
                    Ok(StackVal::Chunk(Some(SpannedStmt { line, file, stmt })))
                } else {
                    Ok(StackVal::Chunk(None))
                }
            }

            // Rules 8..15: statement -> directive | defblock | anonblock | capture | macro | use | view | rawperl
            8..=15 => Ok(v.pop().unwrap()),

            // Rule 16: statement -> expr
            16 => {
                let expr = v.pop().unwrap().into_expr();
                Ok(StackVal::Stmt(Some(Stmt::Get(expr))))
            }

            // Rule 17: statement -> META metadata
            17 => {
                if let StackVal::MetaList(list) = v.pop().unwrap() {
                    self.metadata.extend(list);
                }
                Ok(StackVal::Stmt(None))
            }

            // Rule 18: statement -> /* empty */
            18 => Ok(StackVal::Stmt(None)),

            // Rule 19: directive -> setlist
            19 => {
                let StackVal::SetList(assigns) = v.pop().unwrap() else {
                    return Ok(StackVal::Stmt(None));
                };
                Ok(StackVal::Stmt(Some(Stmt::Set {
                    assigns,
                    is_default: false,
                })))
            }

            // Rules 20..25: directive -> atomdir | condition | switch | loop | try | perl
            20..=25 => Ok(v.pop().unwrap()),

            // Rule 26: atomexpr -> expr
            26 => {
                let expr = v.pop().unwrap().into_expr();
                Ok(StackVal::Stmt(Some(Stmt::Get(expr))))
            }

            // Rule 27: atomexpr -> atomdir
            27 => Ok(v.pop().unwrap()),

            // Rule 28: atomdir -> GET expr
            28 => {
                let expr = v.pop().unwrap().into_expr();
                Ok(StackVal::Stmt(Some(Stmt::Get(expr))))
            }

            // Rule 29: atomdir -> CALL expr
            29 => {
                let expr = v.pop().unwrap().into_expr();
                Ok(StackVal::Stmt(Some(Stmt::Call(expr))))
            }

            // Rule 30: atomdir -> SET setlist
            30 => {
                let StackVal::SetList(assigns) = v.pop().unwrap() else {
                    return Ok(StackVal::Stmt(None));
                };
                Ok(StackVal::Stmt(Some(Stmt::Set {
                    assigns,
                    is_default: false,
                })))
            }

            // Rule 31: atomdir -> DEFAULT setlist
            31 => {
                let StackVal::SetList(assigns) = v.pop().unwrap() else {
                    return Ok(StackVal::Stmt(None));
                };
                Ok(StackVal::Stmt(Some(Stmt::Set {
                    assigns,
                    is_default: true,
                })))
            }

            // Rule 32: atomdir -> INSERT nameargs
            32 => {
                let na = v.pop().unwrap().into_nameargs();
                Ok(StackVal::Stmt(Some(Stmt::Insert {
                    files: na.names,
                    args: na.args,
                })))
            }

            // Rule 33: atomdir -> INCLUDE nameargs
            33 => {
                let na = v.pop().unwrap().into_nameargs();
                Ok(StackVal::Stmt(Some(Stmt::Include {
                    files: na.names,
                    args: na.args,
                })))
            }

            // Rule 34: atomdir -> PROCESS nameargs
            34 => {
                let na = v.pop().unwrap().into_nameargs();
                Ok(StackVal::Stmt(Some(Stmt::Process {
                    files: na.names,
                    args: na.args,
                })))
            }

            // Rule 35: atomdir -> THROW nameargs
            35 => {
                let na = v.pop().unwrap().into_nameargs();
                Ok(StackVal::Stmt(Some(Stmt::Throw {
                    types: na.names,
                    args: na.args,
                })))
            }

            // Rule 36: atomdir -> RETURN
            36 => Ok(StackVal::Stmt(Some(Stmt::Return))),

            // Rule 37: atomdir -> STOP
            37 => Ok(StackVal::Stmt(Some(Stmt::Stop))),

            // Rule 38: atomdir -> CLEAR
            38 => Ok(StackVal::Stmt(Some(Stmt::Clear))),

            // Rule 39: atomdir -> LAST
            39 => Ok(StackVal::Stmt(Some(Stmt::Last {
                label: self.block_label(),
            }))),

            // Rule 40: atomdir -> NEXT
            40 => Ok(StackVal::Stmt(Some(Stmt::Next {
                label: self.block_label(),
                is_for: self.in_block_top("FOR"),
            }))),

            // Rule 41: atomdir -> DEBUG nameargs
            41 => {
                let na = v.pop().unwrap().into_nameargs();
                let first_is_on_off = na.names.first().and_then(|e| match e {
                    Expr::Str(s) if s == "on" => Some(true),
                    Expr::Str(s) if s == "off" => Some(false),
                    _ => None,
                });
                if let Some(on) = first_is_on_off {
                    self.parser.debug_dirs = on;
                    self.debug_dirs = on;
                    Ok(StackVal::Stmt(Some(Stmt::Debug {
                        names: na.names,
                        args: na.args,
                    })))
                } else if self.parser.debug_dirs {
                    Ok(StackVal::Stmt(Some(Stmt::Debug {
                        names: na.names,
                        args: na.args,
                    })))
                } else {
                    Ok(StackVal::Stmt(None))
                }
            }

            // Rules 42..43: atomdir -> wrapper | filter
            42 | 43 => Ok(v.pop().unwrap()),

            // Rule 44: condition -> IF expr ';' block else END
            44 => {
                let _end = v.pop();
                let StackVal::Else { elsifs, else_block } = v.pop().unwrap() else {
                    unreachable!()
                };
                let block = v.pop().unwrap().into_block();
                let _semi = v.pop();
                let expr = v.pop().unwrap().into_expr();
                let mut branches = vec![(expr, block)];
                branches.extend(elsifs);
                Ok(StackVal::Stmt(Some(Stmt::If {
                    branches,
                    else_block,
                })))
            }

            // Rule 45: condition -> atomexpr IF expr
            45 => {
                let expr = v.pop().unwrap().into_expr();
                let _if = v.pop();
                let block = v.pop().unwrap().into_block();
                Ok(StackVal::Stmt(Some(Stmt::If {
                    branches: vec![(expr, block)],
                    else_block: None,
                })))
            }

            // Rule 46: condition -> UNLESS expr ';' block else END
            46 => {
                let _end = v.pop();
                let StackVal::Else { elsifs, else_block } = v.pop().unwrap() else {
                    unreachable!()
                };
                let block = v.pop().unwrap().into_block();
                let _semi = v.pop();
                let expr = v.pop().unwrap().into_expr();
                let mut branches = vec![(Expr::Not(Box::new(expr)), block)];
                branches.extend(elsifs);
                Ok(StackVal::Stmt(Some(Stmt::If {
                    branches,
                    else_block,
                })))
            }

            // Rule 47: condition -> atomexpr UNLESS expr
            47 => {
                let expr = v.pop().unwrap().into_expr();
                let _unless = v.pop();
                let block = v.pop().unwrap().into_block();
                Ok(StackVal::Stmt(Some(Stmt::If {
                    branches: vec![(Expr::Not(Box::new(expr)), block)],
                    else_block: None,
                })))
            }

            // Rule 48: else -> ELSIF expr ';' block else
            48 => {
                let StackVal::Else {
                    mut elsifs,
                    else_block,
                } = v.pop().unwrap()
                else {
                    unreachable!()
                };
                let block = v.pop().unwrap().into_block();
                let _semi = v.pop();
                let expr = v.pop().unwrap().into_expr();
                elsifs.insert(0, (expr, block));
                Ok(StackVal::Else { elsifs, else_block })
            }

            // Rule 49: else -> ELSE ';' block
            49 => {
                let block = v.pop().unwrap().into_block();
                Ok(StackVal::Else {
                    elsifs: Vec::new(),
                    else_block: Some(block),
                })
            }

            // Rule 50: else -> /* NULL */
            50 => Ok(StackVal::Else {
                elsifs: Vec::new(),
                else_block: None,
            }),

            // Rule 51: switch -> SWITCH expr ';' block case END
            51 => {
                let _end = v.pop();
                let StackVal::Case {
                    cases,
                    default_block,
                } = v.pop().unwrap()
                else {
                    unreachable!()
                };
                let _ignored_block = v.pop();
                let _semi = v.pop();
                let expr = v.pop().unwrap().into_expr();
                Ok(StackVal::Stmt(Some(Stmt::Switch {
                    expr,
                    cases,
                    default_block,
                })))
            }

            // Rule 52: case -> CASE term ';' block case
            52 => {
                let StackVal::Case {
                    mut cases,
                    default_block,
                } = v.pop().unwrap()
                else {
                    unreachable!()
                };
                let block = v.pop().unwrap().into_block();
                let _semi = v.pop();
                let term = v.pop().unwrap().into_expr();
                cases.insert(0, (term, block));
                Ok(StackVal::Case {
                    cases,
                    default_block,
                })
            }

            // Rule 53: case -> CASE DEFAULT ';' block
            53 => {
                let block = v.pop().unwrap().into_block();
                Ok(StackVal::Case {
                    cases: Vec::new(),
                    default_block: Some(block),
                })
            }

            // Rule 54: case -> CASE ';' block
            54 => {
                let block = v.pop().unwrap().into_block();
                Ok(StackVal::Case {
                    cases: Vec::new(),
                    default_block: Some(block),
                })
            }

            // Rule 55: case -> /* NULL */
            55 => Ok(StackVal::Case {
                cases: Vec::new(),
                default_block: None,
            }),

            // Rule 56: @1-3 -> /* enter FOR */
            56 => {
                self.enter_block("FOR");
                Ok(StackVal::None)
            }

            // Rule 57: loop -> FOR loopvar ';' @1-3 block END
            57 => {
                let _end = v.pop();
                let block = v.pop().unwrap().into_block();
                let _marker = v.pop();
                let _semi = v.pop();
                let StackVal::LoopVar { var, list, args } = v.pop().unwrap() else {
                    unreachable!()
                };
                let label = self.leave_block();
                Ok(StackVal::Stmt(Some(Stmt::Foreach {
                    loop_var: var,
                    list,
                    args,
                    block,
                    label,
                })))
            }

            // Rule 58: loop -> atomexpr FOR loopvar
            58 => {
                let StackVal::LoopVar { var, list, args } = v.pop().unwrap() else {
                    unreachable!()
                };
                let _for = v.pop();
                let block = v.pop().unwrap().into_block();
                Ok(StackVal::Stmt(Some(Stmt::Foreach {
                    loop_var: var,
                    list,
                    args,
                    block,
                    label: None,
                })))
            }

            // Rule 59: @2-3 -> /* enter WHILE */
            59 => {
                self.enter_block("WHILE");
                Ok(StackVal::None)
            }

            // Rule 60: loop -> WHILE expr ';' @2-3 block END
            60 => {
                let _end = v.pop();
                let block = v.pop().unwrap().into_block();
                let _marker = v.pop();
                let _semi = v.pop();
                let expr = v.pop().unwrap().into_expr();
                let label = self.leave_block();
                Ok(StackVal::Stmt(Some(Stmt::While { expr, block, label })))
            }

            // Rule 61: loop -> atomexpr WHILE expr
            61 => {
                let expr = v.pop().unwrap().into_expr();
                let _while = v.pop();
                let block = v.pop().unwrap().into_block();
                Ok(StackVal::Stmt(Some(Stmt::While {
                    expr,
                    block,
                    label: None,
                })))
            }

            // Rule 62 & 63: loopvar -> IDENT ASSIGN term args | IDENT IN term args
            62 | 63 => {
                let args = v.pop().unwrap().into_args();
                let list = v.pop().unwrap().into_expr();
                let _op = v.pop();
                let ident = v.pop().unwrap().into_token();
                Ok(StackVal::LoopVar {
                    var: Some(ident),
                    list,
                    args,
                })
            }

            // Rule 64: loopvar -> term args
            64 => {
                let args = v.pop().unwrap().into_args();
                let list = v.pop().unwrap().into_expr();
                Ok(StackVal::LoopVar {
                    var: None,
                    list,
                    args,
                })
            }

            // Rule 65: wrapper -> WRAPPER nameargs ';' block END
            65 => {
                let _end = v.pop();
                let block = v.pop().unwrap().into_block();
                let _semi = v.pop();
                let na = v.pop().unwrap().into_nameargs();
                Ok(StackVal::Stmt(Some(Stmt::Wrapper {
                    files: na.names,
                    args: na.args,
                    block,
                })))
            }

            // Rule 66: wrapper -> atomexpr WRAPPER nameargs
            66 => {
                let na = v.pop().unwrap().into_nameargs();
                let _wrapper = v.pop();
                let block = v.pop().unwrap().into_block();
                Ok(StackVal::Stmt(Some(Stmt::Wrapper {
                    files: na.names,
                    args: na.args,
                    block,
                })))
            }

            // Rule 67: try -> TRY ';' block final END
            67 => {
                let _end = v.pop();
                let StackVal::Final {
                    catches,
                    final_block,
                } = v.pop().unwrap()
                else {
                    unreachable!()
                };
                let block = v.pop().unwrap().into_block();
                Ok(StackVal::Stmt(Some(Stmt::Try {
                    block,
                    catches,
                    final_block,
                })))
            }

            // Rule 68: final -> CATCH filename ';' block final
            68 => {
                let StackVal::Final {
                    mut catches,
                    final_block,
                } = v.pop().unwrap()
                else {
                    unreachable!()
                };
                let block = v.pop().unwrap().into_block();
                let _semi = v.pop();
                let filename = v.pop().unwrap().into_token();
                catches.insert(0, (Some(filename), block));
                Ok(StackVal::Final {
                    catches,
                    final_block,
                })
            }

            // Rule 69 & 70: final -> CATCH DEFAULT ';' block final | CATCH ';' block final
            69 | 70 => {
                let StackVal::Final {
                    mut catches,
                    final_block,
                } = v.pop().unwrap()
                else {
                    unreachable!()
                };
                let block = v.pop().unwrap().into_block();
                catches.insert(0, (None, block));
                Ok(StackVal::Final {
                    catches,
                    final_block,
                })
            }

            // Rule 71: final -> FINAL ';' block
            71 => {
                let block = v.pop().unwrap().into_block();
                Ok(StackVal::Final {
                    catches: Vec::new(),
                    final_block: Some(block),
                })
            }

            // Rule 72: final -> /* NULL */
            72 => Ok(StackVal::Final {
                catches: Vec::new(),
                final_block: None,
            }),

            // Rule 73: use -> USE lnameargs
            73 => {
                let na = v.pop().unwrap().into_nameargs();
                Ok(StackVal::Stmt(Some(Stmt::Use {
                    names: na.names,
                    args: na.args,
                    alias: na.alias,
                })))
            }

            // Rule 74: @3-3 -> push_defblock
            74 => {
                self.push_defblock();
                Ok(StackVal::None)
            }

            // Rule 75: view -> VIEW nameargs ';' @3-3 block END
            75 => {
                let _end = v.pop();
                let block = v.pop().unwrap().into_block();
                let _marker = v.pop();
                let _semi = v.pop();
                let na = v.pop().unwrap().into_nameargs();
                let defblocks = self.pop_defblock();
                Ok(StackVal::Stmt(Some(Stmt::View {
                    names: na.names,
                    args: na.args,
                    block,
                    defblocks,
                })))
            }

            // Rule 76 & 78: @4-2 / @5-1 -> inperl++
            76 | 78 => {
                self.inperl += 1;
                Ok(StackVal::None)
            }

            // Rule 77 & 79: perl / rawperl
            77 | 79 => {
                self.inperl = self.inperl.saturating_sub(1);
                Ok(StackVal::Stmt(Some(Stmt::NoPerl)))
            }

            // Rule 80: filter -> FILTER lnameargs ';' block END
            80 => {
                let _end = v.pop();
                let block = v.pop().unwrap().into_block();
                let _semi = v.pop();
                let na = v.pop().unwrap().into_nameargs();
                Ok(StackVal::Stmt(Some(Stmt::Filter {
                    names: na.names,
                    args: na.args,
                    alias: na.alias,
                    block,
                })))
            }

            // Rule 81: filter -> atomexpr FILTER lnameargs
            81 => {
                let na = v.pop().unwrap().into_nameargs();
                let _filter = v.pop();
                let block = v.pop().unwrap().into_block();
                Ok(StackVal::Stmt(Some(Stmt::Filter {
                    names: na.names,
                    args: na.args,
                    alias: na.alias,
                    block,
                })))
            }

            // Rule 82: defblock -> defblockname blockargs ';' template END
            82 => {
                let _end = v.pop();
                let tmpl = v.pop().unwrap().into_block();
                let name = self.defblocks_name_stack.join("/");
                self.defblocks_name_stack.pop();
                self.define_block(name, tmpl);
                Ok(StackVal::Stmt(None))
            }

            // Rule 83: defblockname -> BLOCK blockname
            83 => {
                let bname = v.pop().unwrap().into_token();
                self.defblocks_name_stack.push(bname.clone());
                Ok(StackVal::Token(bname))
            }

            // Rule 84: blockname -> filename
            84 => Ok(v.pop().unwrap()),

            // Rule 85: blockname -> LITERAL
            85 => {
                let lit = v.pop().unwrap().into_token();
                let stripped = if lit.starts_with('\'') && lit.ends_with('\'') && lit.len() >= 2 {
                    lit[1..lit.len() - 1].to_string()
                } else {
                    lit
                };
                Ok(StackVal::Token(stripped))
            }

            // Rule 86: blockargs -> metadata
            86 => Ok(v.pop().unwrap()),

            // Rule 87: blockargs -> /* NULL */
            87 => Ok(StackVal::None),

            // Rule 88: anonblock -> BLOCK blockargs ';' block END
            88 => {
                let _end = v.pop();
                let block = v.pop().unwrap().into_block();
                Ok(StackVal::Stmt(Some(Stmt::AnonBlock(block))))
            }

            // Rule 89: capture -> ident ASSIGN mdir
            89 => {
                let block = v.pop().unwrap().into_block();
                let _assign = v.pop();
                let ident = v.pop().unwrap().into_ident();
                Ok(StackVal::Stmt(Some(Stmt::Capture {
                    target: AssignTarget::Ident(ident),
                    block,
                })))
            }

            // Rule 90: macro -> MACRO IDENT '(' margs ')' mdir
            90 => {
                let block = v.pop().unwrap().into_block();
                let _rparen = v.pop();
                let StackVal::StrList(margs) = v.pop().unwrap() else {
                    unreachable!()
                };
                let _lparen = v.pop();
                let ident = v.pop().unwrap().into_token();
                Ok(StackVal::Stmt(Some(Stmt::Macro {
                    ident,
                    params: Some(margs),
                    block,
                })))
            }

            // Rule 91: macro -> MACRO IDENT mdir
            91 => {
                let block = v.pop().unwrap().into_block();
                let ident = v.pop().unwrap().into_token();
                Ok(StackVal::Stmt(Some(Stmt::Macro {
                    ident,
                    params: None,
                    block,
                })))
            }

            // Rule 92: mdir -> directive
            92 => {
                let b = v.pop().unwrap().into_block();
                Ok(StackVal::Block(b))
            }

            // Rule 93: mdir -> BLOCK ';' block END
            93 => {
                let _end = v.pop();
                let block = v.pop().unwrap().into_block();
                Ok(StackVal::Block(block))
            }

            // Rule 94: margs -> margs IDENT
            94 => {
                let ident = v.pop().unwrap().into_token();
                let StackVal::StrList(mut list) = v.pop().unwrap() else {
                    unreachable!()
                };
                list.push(ident);
                Ok(StackVal::StrList(list))
            }

            // Rule 95: margs -> margs COMMA
            95 => {
                let _comma = v.pop();
                Ok(v.pop().unwrap())
            }

            // Rule 96: margs -> IDENT
            96 => {
                let ident = v.pop().unwrap().into_token();
                Ok(StackVal::StrList(vec![ident]))
            }

            // Rule 97: metadata -> metadata meta
            97 => {
                let StackVal::MetaList(m2) = v.pop().unwrap() else {
                    unreachable!()
                };
                let StackVal::MetaList(mut m1) = v.pop().unwrap() else {
                    unreachable!()
                };
                m1.extend(m2);
                Ok(StackVal::MetaList(m1))
            }

            // Rule 98: metadata -> metadata COMMA
            98 => {
                let _comma = v.pop();
                Ok(v.pop().unwrap())
            }

            // Rule 99: metadata -> meta
            99 => Ok(v.pop().unwrap()),

            // Rule 100: meta -> IDENT ASSIGN LITERAL
            100 => {
                let lit = v.pop().unwrap().into_token();
                let _assign = v.pop();
                let key = v.pop().unwrap().into_token();
                Ok(StackVal::MetaList(vec![(key, unquote_literal(&lit))]))
            }

            // Rule 101: meta -> IDENT ASSIGN '"' TEXT '"'
            101 => {
                let _q2 = v.pop();
                let text = v.pop().unwrap().into_token();
                let _q1 = v.pop();
                let _assign = v.pop();
                let key = v.pop().unwrap().into_token();
                Ok(StackVal::MetaList(vec![(key, text)]))
            }

            // Rule 102: meta -> IDENT ASSIGN NUMBER
            102 => {
                let num = v.pop().unwrap().into_token();
                let _assign = v.pop();
                let key = v.pop().unwrap().into_token();
                Ok(StackVal::MetaList(vec![(key, num)]))
            }

            // Rule 103 & 104: term -> lterm | sterm
            103 | 104 => Ok(v.pop().unwrap()),

            // Rule 105: lterm -> '[' list ']'
            105 => {
                let _rb = v.pop();
                let StackVal::ExprList(items) = v.pop().unwrap() else {
                    unreachable!()
                };
                Ok(StackVal::Expr(Expr::List(items)))
            }

            // Rule 106: lterm -> '[' range ']'
            106 => {
                let _rb = v.pop();
                let StackVal::Range(start, end) = v.pop().unwrap() else {
                    unreachable!()
                };
                Ok(StackVal::Expr(Expr::Range(Box::new(start), Box::new(end))))
            }

            // Rule 107: lterm -> '[' ']'
            107 => Ok(StackVal::Expr(Expr::List(Vec::new()))),

            // Rule 108: lterm -> '{' hash '}'
            108 => {
                let _rc = v.pop();
                let StackVal::Params(entries) = v.pop().unwrap() else {
                    unreachable!()
                };
                Ok(StackVal::Expr(Expr::Hash(entries)))
            }

            // Rule 109: sterm -> ident
            109 => {
                let ident = v.pop().unwrap().into_ident();
                let expr = self.make_ident_expr(ident);
                Ok(StackVal::Expr(expr))
            }

            // Rule 110: sterm -> REF ident
            110 => {
                let ident = v.pop().unwrap().into_ident();
                Ok(StackVal::Expr(Expr::IdentRef(ident)))
            }

            // Rule 111: sterm -> '"' quoted '"'
            111 => {
                let _q2 = v.pop();
                let StackVal::ExprList(items) = v.pop().unwrap() else {
                    unreachable!()
                };
                Ok(StackVal::Expr(Expr::Quoted(items)))
            }

            // Rule 112: sterm -> LITERAL
            112 => {
                let lit = v.pop().unwrap().into_token();
                Ok(StackVal::Expr(Expr::Str(unquote_literal(&lit))))
            }

            // Rule 113: sterm -> NUMBER
            113 => {
                let num = v.pop().unwrap().into_token();
                Ok(StackVal::Expr(parse_number_token(&num)))
            }

            // Rule 114: list -> list term
            114 => {
                let term = v.pop().unwrap().into_expr();
                let StackVal::ExprList(mut list) = v.pop().unwrap() else {
                    unreachable!()
                };
                list.push(term);
                Ok(StackVal::ExprList(list))
            }

            // Rule 115: list -> list COMMA
            115 => {
                let _comma = v.pop();
                Ok(v.pop().unwrap())
            }

            // Rule 116: list -> term
            116 => {
                let term = v.pop().unwrap().into_expr();
                Ok(StackVal::ExprList(vec![term]))
            }

            // Rule 117: range -> sterm TO sterm
            117 => {
                let end = v.pop().unwrap().into_expr();
                let _to = v.pop();
                let start = v.pop().unwrap().into_expr();
                Ok(StackVal::Range(start, end))
            }

            // Rule 118: hash -> params
            118 => Ok(v.pop().unwrap()),

            // Rule 119: hash -> /* NULL */
            119 => Ok(StackVal::Params(Vec::new())),

            // Rule 120: params -> params param
            120 => {
                let StackVal::Params(p2) = v.pop().unwrap() else {
                    unreachable!()
                };
                let StackVal::Params(mut p1) = v.pop().unwrap() else {
                    unreachable!()
                };
                p1.extend(p2);
                Ok(StackVal::Params(p1))
            }

            // Rule 121: params -> params COMMA
            121 => {
                let _comma = v.pop();
                Ok(v.pop().unwrap())
            }

            // Rule 122: params -> param
            122 => Ok(v.pop().unwrap()),

            // Rule 123: param -> LITERAL ASSIGN expr
            123 => {
                let expr = v.pop().unwrap().into_expr();
                let _assign = v.pop();
                let lit = v.pop().unwrap().into_token();
                Ok(StackVal::Params(vec![(
                    Expr::Str(unquote_literal(&lit)),
                    expr,
                )]))
            }

            // Rule 124: param -> item ASSIGN expr
            124 => {
                let expr = v.pop().unwrap().into_expr();
                let _assign = v.pop();
                let item = v.pop().unwrap().into_expr();
                Ok(StackVal::Params(vec![(item, expr)]))
            }

            // Rule 125: ident -> ident DOT node
            125 => {
                let node = v.pop().unwrap().into_ident();
                let _dot = v.pop();
                let mut ident = v.pop().unwrap().into_ident();
                ident.extend(node);
                Ok(StackVal::Ident(ident))
            }

            // Rule 126: ident -> ident DOT NUMBER
            126 => {
                let num = v.pop().unwrap().into_token();
                let _dot = v.pop();
                let mut ident = v.pop().unwrap().into_ident();
                for part in num.split('.') {
                    ident.push(IdentNode {
                        key: Expr::Str(part.to_string()),
                        args: None,
                    });
                }
                Ok(StackVal::Ident(ident))
            }

            // Rule 127: ident -> node
            127 => Ok(v.pop().unwrap()),

            // Rule 128: node -> item
            128 => {
                let item = v.pop().unwrap().into_expr();
                Ok(StackVal::Ident(vec![IdentNode {
                    key: item,
                    args: None,
                }]))
            }

            // Rule 129: node -> item '(' args ')'
            129 => {
                let _rp = v.pop();
                let args = v.pop().unwrap().into_args();
                let _lp = v.pop();
                let item = v.pop().unwrap().into_expr();
                let opt_args = if args.is_empty() { None } else { Some(args) };
                Ok(StackVal::Ident(vec![IdentNode {
                    key: item,
                    args: opt_args,
                }]))
            }

            // Rule 130: item -> IDENT
            130 => {
                let ident = v.pop().unwrap().into_token();
                Ok(StackVal::Expr(Expr::Str(ident)))
            }

            // Rule 131: item -> '${' sterm '}'
            131 => {
                let _rc = v.pop();
                let sterm = v.pop().unwrap().into_expr();
                Ok(StackVal::Expr(sterm))
            }

            // Rule 132: item -> '$' IDENT
            132 => {
                let ident = v.pop().unwrap().into_token();
                if self.parser.current_style().v1dollar {
                    Ok(StackVal::Expr(Expr::Str(ident)))
                } else {
                    let expr = self.make_ident_expr(vec![IdentNode {
                        key: Expr::Str(ident),
                        args: None,
                    }]);
                    Ok(StackVal::Expr(expr))
                }
            }

            // Rule 133: expr -> expr BINOP expr
            133 => {
                let right = v.pop().unwrap().into_expr();
                let op_str = v.pop().unwrap().into_token();
                let left = v.pop().unwrap().into_expr();
                let op = match op_str.as_str() {
                    "-" => BinOp::Sub,
                    "*" => BinOp::Mul,
                    "%" => BinOp::Mod,
                    _ => BinOp::Add,
                };
                Ok(StackVal::Expr(perl_binop(op, left, right)))
            }

            // Rule 134: expr -> expr '/' expr
            134 => {
                let right = v.pop().unwrap().into_expr();
                let _op = v.pop();
                let left = v.pop().unwrap().into_expr();
                Ok(StackVal::Expr(perl_binop(BinOp::Div, left, right)))
            }

            // Rule 135: expr -> expr '+' expr
            135 => {
                let right = v.pop().unwrap().into_expr();
                let _op = v.pop();
                let left = v.pop().unwrap().into_expr();
                Ok(StackVal::Expr(perl_binop(BinOp::Add, left, right)))
            }

            // Rule 136: expr -> expr DIV expr
            136 => {
                let right = v.pop().unwrap().into_expr();
                let _op = v.pop();
                let left = v.pop().unwrap().into_expr();
                Ok(StackVal::Expr(perl_binop(BinOp::IntDiv, left, right)))
            }

            // Rule 137: expr -> expr MOD expr
            137 => {
                let right = v.pop().unwrap().into_expr();
                let _op = v.pop();
                let left = v.pop().unwrap().into_expr();
                Ok(StackVal::Expr(perl_binop(BinOp::Mod, left, right)))
            }

            // Rule 138: expr -> expr CMPOP expr
            138 => {
                let right = v.pop().unwrap().into_expr();
                let op_str = v.pop().unwrap().into_token();
                let left = v.pop().unwrap().into_expr();
                let op = match op_str.as_str() {
                    "==" => BinOp::Eq,
                    "!=" => BinOp::Ne,
                    "<" => BinOp::Lt,
                    ">" => BinOp::Gt,
                    "<=" => BinOp::Le,
                    ">=" => BinOp::Ge,
                    _ => BinOp::Eq,
                };
                Ok(StackVal::Expr(perl_binop(op, left, right)))
            }

            // Rule 139: expr -> expr CAT expr
            139 => {
                let right = v.pop().unwrap().into_expr();
                let _op = v.pop();
                let left = v.pop().unwrap().into_expr();
                Ok(StackVal::Expr(perl_binop(BinOp::Cat, left, right)))
            }

            // Rule 140: expr -> expr AND expr
            140 => {
                let right = v.pop().unwrap().into_expr();
                let _op = v.pop();
                let left = v.pop().unwrap().into_expr();
                Ok(StackVal::Expr(perl_binop(BinOp::And, left, right)))
            }

            // Rule 141: expr -> expr OR expr
            141 => {
                let right = v.pop().unwrap().into_expr();
                let _op = v.pop();
                let left = v.pop().unwrap().into_expr();
                Ok(StackVal::Expr(perl_binop(BinOp::Or, left, right)))
            }

            // Rule 142: expr -> NOT expr
            142 => {
                let expr = v.pop().unwrap().into_expr();
                Ok(StackVal::Expr(perl_not(expr)))
            }

            // Rule 143: expr -> expr '?' expr ':' expr
            143 => {
                let else_expr = v.pop().unwrap().into_expr();
                let _colon = v.pop();
                let then_expr = v.pop().unwrap().into_expr();
                let _qmark = v.pop();
                let cond = v.pop().unwrap().into_expr();
                Ok(StackVal::Expr(Expr::Ternary {
                    cond: Box::new(cond),
                    then_expr: Box::new(then_expr),
                    else_expr: Box::new(else_expr),
                }))
            }

            // Rule 144: expr -> '(' assign ')'
            144 => {
                let _rp = v.pop();
                let StackVal::SetList(mut assigns) = v.pop().unwrap() else {
                    unreachable!()
                };
                let (target, expr) = assigns.remove(0);
                Ok(StackVal::Expr(Expr::Assign {
                    target: Box::new(target),
                    expr: Box::new(expr),
                }))
            }

            // Rule 145: expr -> '(' expr ')'
            145 => {
                let _rp = v.pop();
                let expr = v.pop().unwrap();
                Ok(match expr {
                    StackVal::Expr(e @ (Expr::BinOp { .. } | Expr::Not(_)))
                        if !matches!(
                            e,
                            Expr::BinOp {
                                op: BinOp::IntDiv,
                                ..
                            }
                        ) =>
                    {
                        StackVal::Expr(Expr::Paren(Box::new(e)))
                    }
                    other => other,
                })
            }

            // Rule 146: expr -> term
            146 => Ok(v.pop().unwrap()),

            // Rule 147: setlist -> setlist assign
            147 => {
                let StackVal::SetList(a2) = v.pop().unwrap() else {
                    unreachable!()
                };
                let StackVal::SetList(mut a1) = v.pop().unwrap() else {
                    unreachable!()
                };
                a1.extend(a2);
                Ok(StackVal::SetList(a1))
            }

            // Rule 148: setlist -> setlist COMMA
            148 => {
                let _comma = v.pop();
                Ok(v.pop().unwrap())
            }

            // Rule 149: setlist -> assign
            149 => Ok(v.pop().unwrap()),

            // Rule 150: assign -> ident ASSIGN expr
            150 => {
                let expr = v.pop().unwrap().into_expr();
                let _assign = v.pop();
                let ident = v.pop().unwrap().into_ident();
                Ok(StackVal::SetList(vec![(AssignTarget::Ident(ident), expr)]))
            }

            // Rule 151: assign -> LITERAL ASSIGN expr
            151 => {
                let expr = v.pop().unwrap().into_expr();
                let _assign = v.pop();
                let lit = v.pop().unwrap().into_token();
                Ok(StackVal::SetList(vec![(
                    AssignTarget::Expr(Expr::Str(unquote_literal(&lit))),
                    expr,
                )]))
            }

            // Rule 152: args -> args expr
            152 => {
                let expr = v.pop().unwrap().into_expr();
                let mut args = v.pop().unwrap().into_args();
                args.positional.push(expr);
                Ok(StackVal::Args(args))
            }

            // Rule 153: args -> args param
            153 => {
                let StackVal::Params(params) = v.pop().unwrap() else {
                    unreachable!()
                };
                let mut args = v.pop().unwrap().into_args();
                for (k, val) in params {
                    args.named.push(NamedArg::KeyValue(k, val));
                }
                Ok(StackVal::Args(args))
            }

            // Rule 154: args -> args ident ASSIGN expr
            154 => {
                let expr = v.pop().unwrap().into_expr();
                let _assign = v.pop();
                let ident = v.pop().unwrap().into_ident();
                let mut args = v.pop().unwrap().into_args();
                args.named.push(NamedArg::Assign(ident, expr));
                Ok(StackVal::Args(args))
            }

            // Rule 155: args -> args COMMA
            155 => {
                let _comma = v.pop();
                Ok(v.pop().unwrap())
            }

            // Rule 156: args -> /* init */
            156 => Ok(StackVal::Args(ArgsSpec::default())),

            // Rule 157: lnameargs -> lvalue ASSIGN nameargs
            157 => {
                let mut na = v.pop().unwrap().into_nameargs();
                let _assign = v.pop();
                let StackVal::AssignTarget(target) = v.pop().unwrap() else {
                    unreachable!()
                };
                na.alias = Some(target);
                Ok(StackVal::NameArgs(na))
            }

            // Rule 158: lnameargs -> nameargs
            158 => Ok(v.pop().unwrap()),

            // Rule 159: lvalue -> item
            159 => {
                let item = v.pop().unwrap().into_expr();
                Ok(StackVal::AssignTarget(AssignTarget::Expr(item)))
            }

            // Rule 160: lvalue -> '"' quoted '"'
            160 => {
                let _q2 = v.pop();
                let StackVal::ExprList(items) = v.pop().unwrap() else {
                    unreachable!()
                };
                Ok(StackVal::AssignTarget(AssignTarget::Expr(Expr::Quoted(
                    items,
                ))))
            }

            // Rule 161: lvalue -> LITERAL
            161 => {
                let lit = v.pop().unwrap().into_token();
                Ok(StackVal::AssignTarget(AssignTarget::Expr(Expr::Str(
                    unquote_literal(&lit),
                ))))
            }

            // Rule 162: nameargs -> '$' ident args
            162 => {
                let args = v.pop().unwrap().into_args();
                let ident = v.pop().unwrap().into_ident();
                let expr = self.make_ident_expr(ident);
                Ok(StackVal::NameArgs(NameArgs {
                    names: vec![expr],
                    args,
                    alias: None,
                }))
            }

            // Rule 163: nameargs -> names args
            163 => {
                let args = v.pop().unwrap().into_args();
                let StackVal::ExprList(names) = v.pop().unwrap() else {
                    unreachable!()
                };
                Ok(StackVal::NameArgs(NameArgs {
                    names,
                    args,
                    alias: None,
                }))
            }

            // Rule 164: nameargs -> names '(' args ')'
            164 => {
                let _rp = v.pop();
                let args = v.pop().unwrap().into_args();
                let _lp = v.pop();
                let StackVal::ExprList(names) = v.pop().unwrap() else {
                    unreachable!()
                };
                Ok(StackVal::NameArgs(NameArgs {
                    names,
                    args,
                    alias: None,
                }))
            }

            // Rule 165: names -> names '+' name
            165 => {
                let name = v.pop().unwrap().into_expr();
                let _plus = v.pop();
                let StackVal::ExprList(mut names) = v.pop().unwrap() else {
                    unreachable!()
                };
                names.push(name);
                Ok(StackVal::ExprList(names))
            }

            // Rule 166: names -> name
            166 => {
                let name = v.pop().unwrap().into_expr();
                Ok(StackVal::ExprList(vec![name]))
            }

            // Rule 167: name -> '"' quoted '"'
            167 => {
                let _q2 = v.pop();
                let StackVal::ExprList(items) = v.pop().unwrap() else {
                    unreachable!()
                };
                Ok(StackVal::Expr(Expr::Quoted(items)))
            }

            // Rule 168: name -> filename
            168 => {
                let fname = v.pop().unwrap().into_token();
                Ok(StackVal::Expr(Expr::Str(fname)))
            }

            // Rule 169: name -> LITERAL
            169 => {
                let lit = v.pop().unwrap().into_token();
                Ok(StackVal::Expr(Expr::Str(unquote_literal(&lit))))
            }

            // Rule 170: filename -> filename DOT filepart
            170 => {
                let part = v.pop().unwrap().into_token();
                let _dot = v.pop();
                let base = v.pop().unwrap().into_token();
                Ok(StackVal::Token(format!("{}.{}", base, part)))
            }

            // Rule 171..174: filename -> filepart, filepart -> FILENAME | IDENT | NUMBER
            171..=174 => Ok(v.pop().unwrap()),

            // Rule 175: quoted -> quoted quotable
            175 => {
                let StackVal::OptExpr(opt_e) = v.pop().unwrap() else {
                    unreachable!()
                };
                let StackVal::ExprList(mut list) = v.pop().unwrap() else {
                    unreachable!()
                };
                if let Some(e) = opt_e {
                    list.push(e);
                }
                Ok(StackVal::ExprList(list))
            }

            // Rule 176: quoted -> /* NULL */
            176 => Ok(StackVal::ExprList(Vec::new())),

            // Rule 177: quotable -> ident
            177 => {
                let ident = v.pop().unwrap().into_ident();
                let expr = self.make_ident_expr(ident);
                Ok(StackVal::OptExpr(Some(expr)))
            }

            // Rule 178: quotable -> TEXT
            178 => {
                let text = v.pop().unwrap().into_token();
                Ok(StackVal::OptExpr(Some(Expr::Str(text))))
            }

            // Rule 179: quotable -> ';'
            179 => Ok(StackVal::OptExpr(None)),

            _ => Err(format!("unknown rule index: {}", rule_idx)),
        }
    }
}
