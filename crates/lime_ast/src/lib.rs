use lime_span::Span;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NodeId(pub u32);

#[derive(Clone, Debug)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct TypeExpr {
    pub kind: TypeExprKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum TypeExprKind {
    Named(String),
    Array { elem: Box<TypeExpr>, size: u64 },
}

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
    Enum(EnumDecl),
    Main(MainBlock),
    Extern(ExternDecl),
    Use(Ident),
    Import(Ident),
}

#[derive(Debug)]
pub struct EnumDecl {
    pub public: bool,
    pub name: Ident,
    pub variants: Vec<Ident>,
    pub span: Span,
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

    Let {
        mutable: bool,
        name: Ident,
        ty: Option<TypeExpr>,
        init: Expr,
        span: Span,
    },

    Assign { target: Expr, value: Expr, span: Span },

    Expr(Expr),
    If(IfStmt),
    While { cond: Expr, body: Block, span: Span },
    For(ForStmt),
    ForEach(ForEachStmt),
    Return { value: Option<Expr>, span: Span },
    Break(Span),
    Continue(Span),
    Block(Block),
}

#[derive(Debug)]
pub struct ForEachStmt {
    pub var: Ident,
    pub iterable: Expr,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug)]
pub struct IfStmt {
    pub cond: Expr,
    pub then_block: Block,

    pub else_ifs: Vec<(Expr, Block)>,
    pub else_block: Option<Block>,
    pub span: Span,
}

#[derive(Debug)]
pub struct ForStmt {
    pub var: Ident,
    pub start: Expr,
    pub end: Expr,

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

    Int { value: u128, suffix: Option<String> },
    Float { value: f64, suffix: Option<String> },
    Str(String),
    Char(char),
    Bool(bool),

    Name(String),
    Binary { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    Unary { op: UnOp, operand: Box<Expr> },

    Call { callee: Box<Expr>, args: Vec<Expr> },

    Field { base: Box<Expr>, field: Ident },

    StructLit { name: Ident, fields: Vec<StructLitField> },

    ArrayLit(Vec<Expr>),
    Index { base: Box<Expr>, index: Box<Expr> },
}

#[derive(Debug)]
pub struct StructLitField {
    pub name: Ident,
    pub value: Expr,
    pub span: Span,
}

impl Expr {
    pub fn is_lvalue(&self) -> bool {
        matches!(
            self.kind,
            ExprKind::Name(_) | ExprKind::Field { .. } | ExprKind::Index { .. }
        )
    }
}
