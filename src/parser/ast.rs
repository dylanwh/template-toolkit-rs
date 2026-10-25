use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VariableTrace {
    pub children: HashMap<String, VariableTrace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDocument {
    pub block: Block,
    pub defblocks: HashMap<String, Block>,
    pub metadata: HashMap<String, String>,
    pub variables: HashMap<String, VariableTrace>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Block {
    pub stmts: Vec<SpannedStmt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpannedStmt {
    pub line: Option<String>,
    pub file: Option<String>,
    pub stmt: Stmt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Stmt {
    Text(String),
    Get(Expr),
    Call(Expr),
    Set {
        assigns: Vec<(AssignTarget, Expr)>,
        is_default: bool,
    },
    Insert {
        files: Vec<Expr>,
        args: ArgsSpec,
    },
    Include {
        files: Vec<Expr>,
        args: ArgsSpec,
    },
    Process {
        files: Vec<Expr>,
        args: ArgsSpec,
    },
    Throw {
        types: Vec<Expr>,
        args: ArgsSpec,
    },
    Return,
    Stop,
    Clear,
    Last {
        label: Option<String>,
    },
    Next {
        label: Option<String>,
        is_for: bool,
    },
    Debug {
        names: Vec<Expr>,
        args: ArgsSpec,
    },
    Wrapper {
        files: Vec<Expr>,
        args: ArgsSpec,
        block: Block,
    },
    Filter {
        names: Vec<Expr>,
        args: ArgsSpec,
        alias: Option<AssignTarget>,
        block: Block,
    },
    If {
        branches: Vec<(Expr, Block)>,
        else_block: Option<Block>,
    },
    Switch {
        expr: Expr,
        cases: Vec<(Expr, Block)>,
        default_block: Option<Block>,
    },
    Foreach {
        loop_var: Option<String>,
        list: Expr,
        args: ArgsSpec,
        block: Block,
        label: Option<String>,
    },
    While {
        expr: Expr,
        block: Block,
        label: Option<String>,
    },
    Try {
        block: Block,
        catches: Vec<(Option<String>, Block)>,
        final_block: Option<Block>,
    },
    Use {
        names: Vec<Expr>,
        args: ArgsSpec,
        alias: Option<AssignTarget>,
    },
    View {
        names: Vec<Expr>,
        args: ArgsSpec,
        block: Block,
        defblocks: HashMap<String, Block>,
    },
    NoPerl,
    AnonBlock(Block),
    Capture {
        target: AssignTarget,
        block: Block,
    },
    Macro {
        ident: String,
        params: Option<Vec<String>>,
        block: Block,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssignTarget {
    Ident(Vec<IdentNode>),
    Expr(Expr),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentNode {
    pub key: Expr,
    pub args: Option<ArgsSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArgsSpec {
    pub positional: Vec<Expr>,
    pub named: Vec<NamedArg>,
}

impl ArgsSpec {
    pub fn is_empty(&self) -> bool {
        self.positional.is_empty() && self.named.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NameArgs {
    pub names: Vec<Expr>,
    pub args: ArgsSpec,
    pub alias: Option<AssignTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NamedArg {
    KeyValue(Expr, Expr),
    Assign(Vec<IdentNode>, Expr),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expr {
    Str(String),
    Int(i64),
    Float(f64),
    Ident(Vec<IdentNode>),
    IdentRef(Vec<IdentNode>),
    Quoted(Vec<Expr>),
    List(Vec<Expr>),
    Range(Box<Expr>, Box<Expr>),
    Hash(Vec<(Expr, Expr)>),
    BinOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Not(Box<Expr>),
    /// Parenthesised expression (transparent at evaluation time).
    Paren(Box<Expr>),
    Ternary {
        cond: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },
    Assign {
        target: Box<AssignTarget>,
        expr: Box<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    IntDiv,
    Mod,
    Cat,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}
