//! The Lime abstract syntax tree.
//!
//! Nodes carry [`Span`]s for diagnostics. Expressions additionally carry a
//! [`NodeId`] so later passes (type checking, lowering) can attach information
//! to a specific node without mutating the tree. The AST is Lime's own — it is
//! never a view onto C/C++ syntax.

use lime_span::Span;

/// Stable identifier for an expression node, assigned by the parser.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NodeId(pub u32);

/// A name plus the span where it was written.
#[derive(Clone, Debug)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

/// A type as written in source (resolved later by `lime_types`).
#[derive(Clone, Debug)]
pub struct TypeExpr {
    pub name: String,
    pub span: Span,
}

/// One compilation unit: `program <name>` followed by items.
#[derive(Debug)]
pub struct Program {
    pub name: Ident,
    pub header_span: Span,
    pub items: Vec<Item>,
}

#[derive(Debug)]
pub enum Item {
    Function(Function),
    Struct(StructDecl),
    Main(MainBlock),
    Extern(ExternDecl),
    Use(Ident),
    Import(Ident),
}

#[derive(Debug)]
pub struct Function {
    pub public: bool,
    pub name: Ident,
    pub params: Vec<Param>,
    pub ret: Option<TypeExpr>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug)]
pub struct MainBlock {
    pub body: Block,
    pub span: Span,
}

#[derive(Debug)]
pub struct ExternDecl {
    pub name: Ident,
    pub params: Vec<Param>,
    pub variadic: bool,
    pub ret: Option<TypeExpr>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub name: Ident,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Debug)]
pub struct StructDecl {
    pub public: bool,
    pub name: Ident,
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Debug)]
pub struct Field {
    pub public: bool,
    pub name: Ident,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug)]
pub enum Stmt {
    /// `nilai`/`ubah` binding.
    Let {
        mutable: bool,
        name: Ident,
        ty: Option<TypeExpr>,
        init: Expr,
        span: Span,
    },
    /// `lhs = rhs`
    Assign { target: Expr, value: Expr, span: Span },
    /// A bare expression evaluated for effect.
    Expr(Expr),
    If(IfStmt),
    While { cond: Expr, body: Block, span: Span },
    For(ForStmt),
    Return { value: Option<Expr>, span: Span },
    Break(Span),
    Continue(Span),
    Block(Block),
}

#[derive(Debug)]
pub struct IfStmt {
    pub cond: Expr,
    pub then_block: Block,
    /// Zero or more `lain jika` arms.
    pub else_ifs: Vec<(Expr, Block)>,
    pub else_block: Option<Block>,
    pub span: Span,
}

#[derive(Debug)]
pub struct ForStmt {
    pub var: Ident,
    pub start: Expr,
    pub end: Expr,
    /// `true` for `hingga` (inclusive), `false` for `sampai` (exclusive).
    pub inclusive: bool,
    pub body: Block,
    pub span: Span,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

impl BinOp {
    pub fn symbol(self) -> &'static str {
        use BinOp::*;
        match self {
            Add => "+",
            Sub => "-",
            Mul => "*",
            Div => "/",
            Rem => "%",
            Eq => "==",
            Ne => "!=",
            Lt => "<",
            Le => "<=",
            Gt => ">",
            Ge => ">=",
            And => "&&",
            Or => "||",
        }
    }

    pub fn is_comparison(self) -> bool {
        use BinOp::*;
        matches!(self, Eq | Ne | Lt | Le | Gt | Ge)
    }

    pub fn is_logical(self) -> bool {
        matches!(self, BinOp::And | BinOp::Or)
    }

    pub fn is_arithmetic(self) -> bool {
        use BinOp::*;
        matches!(self, Add | Sub | Mul | Div | Rem)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Debug)]
pub struct Expr {
    pub id: NodeId,
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum ExprKind {
    /// Integer literal magnitude + optional explicit suffix type name.
    Int { value: u128, suffix: Option<String> },
    Float { value: f64, suffix: Option<String> },
    Str(String),
    Char(char),
    Bool(bool),
    /// A bare name reference.
    Name(String),
    Binary { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    Unary { op: UnOp, operand: Box<Expr> },
    /// `callee(args...)`
    Call { callee: Box<Expr>, args: Vec<Expr> },
    /// `base.field`
    Field { base: Box<Expr>, field: Ident },
}

impl Expr {
    pub fn is_lvalue(&self) -> bool {
        matches!(self.kind, ExprKind::Name(_) | ExprKind::Field { .. })
    }
}
