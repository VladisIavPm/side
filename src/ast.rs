use crate::value::Value;

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Decl(Decl),
    Stmt(Stmt),
}

#[derive(Debug, Clone)]
pub enum Decl {
    Link { path: String, alias: Option<String> },
    Proc { name: String, params: Vec<String>, body: Vec<Stmt> },
    Form { name: String, fields: Vec<FieldDecl> },
}

#[derive(Debug, Clone)]
pub struct FieldDecl {
    pub name: String,
    pub mutable: bool,
    pub initial: Option<Expr>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Set { name: String, value: Expr, mutable: bool },
    Assign { target: Expr, value: Expr },
    Log(Expr),
    If { condition: Expr, then_block: Vec<Stmt>, else_block: Option<Vec<Stmt>> },
    Loop { condition: Expr, body: Vec<Stmt> },
    Break,
    Wait(Expr),
    Return(Option<Expr>),
    Trap { try_block: Vec<Stmt>, catch_block: Vec<Stmt> },
    ExprStmt(Expr),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Literal(Value),
    List(Vec<Expr>),
    Variable(String),
    Binary {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },
    Unary {
        op: UnOp,
        expr: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
    Field {
        object: Box<Expr>,
        field: String,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    New {
        name: String,
    },
    Entry {
        prompt: Option<Box<Expr>>,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Add, Sub, Mul, Div, Rem,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or,
}

#[derive(Debug, Clone, Copy)]
pub enum UnOp {
    Not,
}