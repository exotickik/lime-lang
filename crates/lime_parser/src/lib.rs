use lime_ast::*;
use lime_diagnostics::{codes, Diagnostic, DiagnosticEngine};
use lime_lexer::{Keyword, LitValue, Token, TokenKind};
use lime_span::{FileId, Span};

pub struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    diags: &'a mut DiagnosticEngine,
    next_node: u32,
    allow_struct_lit: bool,
}

struct Bail;
type PResult<T> = Result<T, Bail>;

impl<'a> Parser<'a> {
    pub fn new(_file: FileId, tokens: Vec<Token>, diags: &'a mut DiagnosticEngine) -> Parser<'a> {
        Parser { tokens, pos: 0, diags, next_node: 0, allow_struct_lit: true }
    }

    fn node_id(&mut self) -> NodeId {
        let id = NodeId(self.next_node);
        self.next_node += 1;
        id
    }

    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn peek_span(&self) -> Span {
        self.tokens[self.pos].span
    }

    fn peek2(&self) -> &TokenKind {
        self.tokens.get(self.pos + 1).map(|t| &t.kind).unwrap_or(&TokenKind::Eof)
    }

    fn at_eof(&self) -> bool {
        matches!(self.peek(), TokenKind::Eof)
    }

    fn bump(&mut self) -> Token {
        let tok = self.tokens[self.pos].clone();
        if !matches!(tok.kind, TokenKind::Eof) {
            self.pos += 1;
        }
        tok
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        if self.peek() == kind {
            self.bump();
            true
        } else {
            false
        }
    }

    fn eat_kw(&mut self, kw: Keyword) -> bool {
        if matches!(self.peek(), TokenKind::Keyword(k) if *k == kw) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn is_kw(&self, kw: Keyword) -> bool {
        matches!(self.peek(), TokenKind::Keyword(k) if *k == kw)
    }

    fn expect(&mut self, kind: &TokenKind) -> PResult<Token> {
        if self.peek() == kind {
            Ok(self.bump())
        } else {
            let found = self.peek().describe();
            let sp = self.peek_span();
            self.diags.emit(
                Diagnostic::error(
                    codes::E_EXPECTED_TOKEN,
                    format!("diharapkan {}, ditemukan {}", kind.describe(), found),
                )
                .primary(sp, format!("diharapkan {} di sini", kind.describe())),
            );
            Err(Bail)
        }
    }

    fn expect_ident(&mut self) -> PResult<Ident> {
        match self.peek().clone() {
            TokenKind::Ident(name) => {
                let sp = self.peek_span();
                self.bump();
                Ok(Ident { name, span: sp })
            }
            other => {
                let sp = self.peek_span();
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_EXPECTED_TOKEN,
                        format!("diharapkan pengenal, ditemukan {}", other.describe()),
                    )
                    .primary(sp, "seharusnya sebuah nama di sini"),
                );
                Err(Bail)
            }
        }
    }

    fn expect_array_size(&mut self) -> PResult<u64> {
        match self.peek().clone() {
            TokenKind::Literal(LitValue::Int { value, .. }) => {
                self.bump();
                Ok(value as u64)
            }
            other => {
                let sp = self.peek_span();
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_EXPECTED_TOKEN,
                        format!("diharapkan ukuran larik (bilangan bulat), ditemukan {}", other.describe()),
                    )
                    .primary(sp, "ukuran larik harus literal bilangan bulat"),
                );
                Err(Bail)
            }
        }
    }

    fn expect_type(&mut self) -> PResult<TypeExpr> {

        match self.peek().clone() {
            TokenKind::Ident(name) => {
                let sp = self.peek_span();
                self.bump();
                Ok(TypeExpr { kind: TypeExprKind::Named(name), span: sp })
            }
            TokenKind::LBracket => {
                let start = self.peek_span();
                self.bump();
                let elem = self.expect_type()?;
                self.expect(&TokenKind::Semicolon)?;
                let size = self.expect_array_size()?;
                let rb = self.expect(&TokenKind::RBracket)?;
                Ok(TypeExpr {
                    kind: TypeExprKind::Array { elem: Box::new(elem), size },
                    span: start.to(rb.span),
                })
            }
            other => {
                let sp = self.peek_span();
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_EXPECTED_TOKEN,
                        format!("diharapkan nama tipe, ditemukan {}", other.describe()),
                    )
                    .primary(sp, "tipe ditulis sebagai nama (mis. 'i32') atau larik '[T; N]'"),
                );
                Err(Bail)
            }
        }
    }

    pub fn parse_program(&mut self) -> Program {

        let (name, header_span) = if self.is_kw(Keyword::Program) {
            let start = self.peek_span();
            self.bump();
            match self.expect_ident() {
                Ok(id) => {
                    let hs = start.to(id.span);
                    (id, hs)
                }
                Err(_) => (
                    Ident { name: "<error>".into(), span: start },
                    start,
                ),
            }
        } else {
            let sp = self.peek_span();
            self.diags.emit(
                Diagnostic::error(
                    codes::E_MISSING_PROGRAM_HEADER,
                    "berkas Lime harus diawali dengan 'program <Nama>'",
                )
                .primary(sp, "tambahkan header program di sini")
                .help("contoh: program Halo"),
            );
            (Ident { name: "<error>".into(), span: sp }, sp)
        };

        let mut items = Vec::new();
        while !self.at_eof() {
            match self.parse_item() {
                Ok(item) => items.push(item),
                Err(_) => self.recover_to_item(),
            }
        }

        Program { name, header_span, items }
    }

    fn parse_item(&mut self) -> PResult<Item> {
        let public = self.eat_kw(Keyword::Publik);
        match self.peek() {
            TokenKind::Keyword(Keyword::Fungsi) => Ok(Item::Function(self.parse_function(public)?)),
            TokenKind::Keyword(Keyword::Tipe) => Ok(Item::Struct(self.parse_struct(public)?)),
            TokenKind::Keyword(Keyword::Enum) => Ok(Item::Enum(self.parse_enum(public)?)),
            TokenKind::Keyword(Keyword::Utama) if !public => Ok(Item::Main(self.parse_main()?)),
            TokenKind::Keyword(Keyword::Ekstern) if !public => {
                Ok(Item::Extern(self.parse_extern()?))
            }
            TokenKind::Keyword(Keyword::Gunakan) if !public => {
                self.bump();
                Ok(Item::Use(self.expect_ident()?))
            }
            TokenKind::Keyword(Keyword::Impor) if !public => {
                self.bump();
                Ok(Item::Import(self.expect_ident()?))
            }
            other => {
                let sp = self.peek_span();
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_EXPECTED_ITEM,
                        format!("diharapkan deklarasi item, ditemukan {}", other.describe()),
                    )
                    .primary(sp, "item: fungsi, tipe, utama, ekstern, gunakan, impor")
                    .note("item tingkat atas dimulai dengan salah satu kata kunci tersebut"),
                );
                Err(Bail)
            }
        }
    }

    fn parse_function(&mut self, public: bool) -> PResult<Function> {
        let start = self.peek_span();
        self.expect(&TokenKind::Keyword(Keyword::Fungsi))?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(&TokenKind::RParen)?;
        let ret = if self.eat(&TokenKind::Arrow) { Some(self.expect_type()?) } else { None };
        let body = self.parse_block()?;
        let span = start.to(body.span);
        Ok(Function { public, name, params, ret, body, span })
    }

    fn parse_params(&mut self) -> PResult<Vec<Param>> {
        let mut params = Vec::new();
        while !matches!(self.peek(), TokenKind::RParen | TokenKind::Eof) {
            let name = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let ty = self.expect_type()?;
            let span = name.span.to(ty.span);
            params.push(Param { name, ty, span });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        Ok(params)
    }

    fn parse_extern(&mut self) -> PResult<ExternDecl> {
        let start = self.peek_span();
        self.expect(&TokenKind::Keyword(Keyword::Ekstern))?;
        self.expect(&TokenKind::Keyword(Keyword::Fungsi))?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LParen)?;
        let mut params = Vec::new();
        let mut variadic = false;
        while !matches!(self.peek(), TokenKind::RParen | TokenKind::Eof) {

            if matches!(self.peek(), TokenKind::Dot)
                && matches!(self.peek2(), TokenKind::Dot)
            {
                self.bump();
                self.bump();
                self.eat(&TokenKind::Dot);
                variadic = true;
                break;
            }
            let pname = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let ty = self.expect_type()?;
            let span = pname.span.to(ty.span);
            params.push(Param { name: pname, ty, span });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        let rp = self.expect(&TokenKind::RParen)?;
        let ret = if self.eat(&TokenKind::Arrow) { Some(self.expect_type()?) } else { None };
        let end = ret.as_ref().map(|t| t.span).unwrap_or(rp.span);
        Ok(ExternDecl { name, params, variadic, ret, span: start.to(end) })
    }

    fn parse_struct(&mut self, public: bool) -> PResult<StructDecl> {
        let start = self.peek_span();
        self.expect(&TokenKind::Keyword(Keyword::Tipe))?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while !matches!(self.peek(), TokenKind::RBrace | TokenKind::Eof) {
            let fpublic = self.eat_kw(Keyword::Publik);
            let fname = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let ty = self.expect_type()?;
            let fspan = fname.span.to(ty.span);
            fields.push(Field { public: fpublic, name: fname, ty, span: fspan });
            self.eat(&TokenKind::Comma);
        }
        let rb = self.expect(&TokenKind::RBrace)?;
        Ok(StructDecl { public, name, fields, span: start.to(rb.span) })
    }

    fn parse_enum(&mut self, public: bool) -> PResult<EnumDecl> {
        let start = self.peek_span();
        self.expect(&TokenKind::Keyword(Keyword::Enum))?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;
        let mut variants = Vec::new();
        while !matches!(self.peek(), TokenKind::RBrace | TokenKind::Eof) {
            let variant = self.expect_ident()?;
            variants.push(variant);
            self.eat(&TokenKind::Comma);
        }
        let rb = self.expect(&TokenKind::RBrace)?;
        Ok(EnumDecl { public, name, variants, span: start.to(rb.span) })
    }

    fn parse_main(&mut self) -> PResult<MainBlock> {
        let start = self.peek_span();
        self.expect(&TokenKind::Keyword(Keyword::Utama))?;
        let body = self.parse_block()?;
        Ok(MainBlock { span: start.to(body.span), body })
    }

    fn parse_block(&mut self) -> PResult<Block> {
        let lb = self.expect(&TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while !matches!(self.peek(), TokenKind::RBrace | TokenKind::Eof) {
            match self.parse_stmt() {
                Ok(s) => stmts.push(s),
                Err(_) => {
                    self.recover_in_block();
                    if matches!(self.peek(), TokenKind::RBrace | TokenKind::Eof) {
                        break;
                    }
                }
            }
        }
        let rb = self.expect(&TokenKind::RBrace)?;
        Ok(Block { stmts, span: lb.span.to(rb.span) })
    }

    fn parse_stmt(&mut self) -> PResult<Stmt> {
        match self.peek() {
            TokenKind::Keyword(Keyword::Nilai) | TokenKind::Keyword(Keyword::Ubah) => {
                self.parse_let()
            }
            TokenKind::Keyword(Keyword::Jika) => Ok(Stmt::If(self.parse_if()?)),
            TokenKind::Keyword(Keyword::Selama) => self.parse_while(),
            TokenKind::Keyword(Keyword::Untuk) => self.parse_for_stmt(),
            TokenKind::Keyword(Keyword::Kembalikan) => self.parse_return(),
            TokenKind::Keyword(Keyword::Berhenti) => {
                let sp = self.peek_span();
                self.bump();
                self.eat(&TokenKind::Semicolon);
                Ok(Stmt::Break(sp))
            }
            TokenKind::Keyword(Keyword::Lanjut) => {
                let sp = self.peek_span();
                self.bump();
                self.eat(&TokenKind::Semicolon);
                Ok(Stmt::Continue(sp))
            }
            TokenKind::LBrace => Ok(Stmt::Block(self.parse_block()?)),
            _ => self.parse_assign_or_expr(),
        }
    }

    fn parse_let(&mut self) -> PResult<Stmt> {
        let start = self.peek_span();
        let mutable = self.is_kw(Keyword::Ubah);
        self.bump();
        let name = self.expect_ident()?;
        let ty = if self.eat(&TokenKind::Colon) { Some(self.expect_type()?) } else { None };
        self.expect(&TokenKind::Eq)?;
        let init = self.parse_expr()?;
        let span = start.to(init.span);
        self.eat(&TokenKind::Semicolon);
        Ok(Stmt::Let { mutable, name, ty, init, span })
    }

    fn parse_return(&mut self) -> PResult<Stmt> {
        let start = self.peek_span();
        self.bump();

        let value = if matches!(
            self.peek(),
            TokenKind::RBrace | TokenKind::Semicolon | TokenKind::Eof
        ) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        let span = value.as_ref().map(|e| start.to(e.span)).unwrap_or(start);
        self.eat(&TokenKind::Semicolon);
        Ok(Stmt::Return { value, span })
    }

    fn parse_if(&mut self) -> PResult<IfStmt> {
        let start = self.peek_span();
        self.expect(&TokenKind::Keyword(Keyword::Jika))?;
        let cond = self.parse_cond_expr()?;
        let then_block = self.parse_block()?;
        let mut else_ifs = Vec::new();
        let mut else_block = None;
        let mut end = then_block.span;
        while self.is_kw(Keyword::Lain) {
            self.bump();
            if self.is_kw(Keyword::Jika) {
                self.bump();
                let c = self.parse_cond_expr()?;
                let b = self.parse_block()?;
                end = b.span;
                else_ifs.push((c, b));
            } else {
                let b = self.parse_block()?;
                end = b.span;
                else_block = Some(b);
                break;
            }
        }
        Ok(IfStmt { cond, then_block, else_ifs, else_block, span: start.to(end) })
    }

    fn parse_while(&mut self) -> PResult<Stmt> {
        let start = self.peek_span();
        self.expect(&TokenKind::Keyword(Keyword::Selama))?;
        let cond = self.parse_cond_expr()?;
        let body = self.parse_block()?;
        let span = start.to(body.span);
        Ok(Stmt::While { cond, body, span })
    }

    fn parse_for_stmt(&mut self) -> PResult<Stmt> {
        let start = self.peek_span();
        self.expect(&TokenKind::Keyword(Keyword::Untuk))?;
        let var = self.expect_ident()?;
        if self.eat_kw(Keyword::Pada) {
            let iterable = self.parse_cond_expr()?;
            let body = self.parse_block()?;
            return Ok(Stmt::ForEach(ForEachStmt {
                var,
                iterable,
                span: start.to(body.span),
                body,
            }));
        }
        self.expect(&TokenKind::Keyword(Keyword::Dari))?;
        let start_expr = self.parse_cond_expr()?;
        let inclusive = if self.eat_kw(Keyword::Hingga) {
            true
        } else if self.eat_kw(Keyword::Sampai) {
            false
        } else {
            let sp = self.peek_span();
            self.diags.emit(
                Diagnostic::error(
                    codes::E_EXPECTED_TOKEN,
                    "diharapkan 'sampai' atau 'hingga' pada perulangan 'untuk'",
                )
                .primary(sp, "batas rentang perulangan")
                .help("'sampai' eksklusif (0..9), 'hingga' inklusif (0..10)"),
            );
            return Err(Bail);
        };
        let end_expr = self.parse_cond_expr()?;
        let body = self.parse_block()?;
        Ok(Stmt::For(ForStmt {
            var,
            start: start_expr,
            end: end_expr,
            inclusive,
            span: start.to(body.span),
            body,
        }))
    }

    fn parse_assign_or_expr(&mut self) -> PResult<Stmt> {
        let expr = self.parse_expr()?;
        if matches!(self.peek(), TokenKind::Eq) {
            self.bump();
            let value = self.parse_expr()?;
            let span = expr.span.to(value.span);
            if !expr.is_lvalue() {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_INVALID_ASSIGN_TARGET,
                        "target penugasan tidak valid",
                    )
                    .primary(expr.span, "hanya nama atau field yang dapat ditetapkan"),
                );
            }
            self.eat(&TokenKind::Semicolon);
            Ok(Stmt::Assign { target: expr, value, span })
        } else {
            self.eat(&TokenKind::Semicolon);
            Ok(Stmt::Expr(expr))
        }
    }

    fn parse_expr(&mut self) -> PResult<Expr> {
        self.parse_bin(0)
    }

    fn parse_cond_expr(&mut self) -> PResult<Expr> {
        let prev = self.allow_struct_lit;
        self.allow_struct_lit = false;
        let result = self.parse_bin(0);
        self.allow_struct_lit = prev;
        result
    }

    fn parse_array_lit(&mut self) -> PResult<Expr> {
        let start = self.peek_span();
        self.expect(&TokenKind::LBracket)?;
        let prev = self.allow_struct_lit;
        self.allow_struct_lit = true;
        let mut elems = Vec::new();
        while !matches!(self.peek(), TokenKind::RBracket | TokenKind::Eof) {
            match self.parse_expr() {
                Ok(e) => elems.push(e),
                Err(e) => {
                    self.allow_struct_lit = prev;
                    return Err(e);
                }
            }
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.allow_struct_lit = prev;
        let rb = self.expect(&TokenKind::RBracket)?;
        let id = self.node_id();
        Ok(Expr { id, kind: ExprKind::ArrayLit(elems), span: start.to(rb.span) })
    }

    fn parse_struct_lit(&mut self, name: Ident) -> PResult<Expr> {
        self.expect(&TokenKind::LBrace)?;
        let prev = self.allow_struct_lit;
        self.allow_struct_lit = true;
        let mut fields = Vec::new();
        while !matches!(self.peek(), TokenKind::RBrace | TokenKind::Eof) {
            let fname = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let value = self.parse_expr()?;
            let fspan = fname.span.to(value.span);
            fields.push(StructLitField { name: fname, value, span: fspan });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.allow_struct_lit = prev;
        let rb = self.expect(&TokenKind::RBrace)?;
        let span = name.span.to(rb.span);
        let id = self.node_id();
        Ok(Expr { id, kind: ExprKind::StructLit { name, fields }, span })
    }

    fn bin_prec(kind: &TokenKind) -> Option<(BinOp, u8)> {
        use TokenKind as T;
        Some(match kind {
            T::PipePipe | T::Keyword(Keyword::Atau) => (BinOp::Or, 1),
            T::AmpAmp | T::Keyword(Keyword::Dan) => (BinOp::And, 2),
            T::EqEq => (BinOp::Eq, 3),
            T::Ne => (BinOp::Ne, 3),
            T::Lt => (BinOp::Lt, 4),
            T::Le => (BinOp::Le, 4),
            T::Gt => (BinOp::Gt, 4),
            T::Ge => (BinOp::Ge, 4),
            T::Plus => (BinOp::Add, 5),
            T::Minus => (BinOp::Sub, 5),
            T::Star => (BinOp::Mul, 6),
            T::Slash => (BinOp::Div, 6),
            T::Percent => (BinOp::Rem, 6),
            _ => return None,
        })
    }

    fn parse_bin(&mut self, min_prec: u8) -> PResult<Expr> {
        let mut lhs = self.parse_unary()?;
        while let Some((op, prec)) = Self::bin_prec(self.peek()) {
            if prec < min_prec {
                break;
            }
            self.bump();

            let rhs = self.parse_bin(prec + 1)?;
            let span = lhs.span.to(rhs.span);
            let id = self.node_id();
            lhs = Expr {
                id,
                kind: ExprKind::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) },
                span,
            };
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> PResult<Expr> {
        let (op, start) = match self.peek() {
            TokenKind::Minus => (Some(UnOp::Neg), self.peek_span()),
            TokenKind::Bang | TokenKind::Keyword(Keyword::Bukan) => {
                (Some(UnOp::Not), self.peek_span())
            }
            _ => (None, self.peek_span()),
        };
        if let Some(op) = op {
            self.bump();
            let operand = self.parse_unary()?;
            let span = start.to(operand.span);
            let id = self.node_id();
            Ok(Expr { id, kind: ExprKind::Unary { op, operand: Box::new(operand) }, span })
        } else {
            self.parse_postfix()
        }
    }

    fn parse_postfix(&mut self) -> PResult<Expr> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek() {
                TokenKind::LParen => {
                    self.bump();
                    let prev = self.allow_struct_lit;
                    self.allow_struct_lit = true;
                    let mut args = Vec::new();
                    while !matches!(self.peek(), TokenKind::RParen | TokenKind::Eof) {
                        match self.parse_expr() {
                            Ok(e) => args.push(e),
                            Err(e) => {
                                self.allow_struct_lit = prev;
                                return Err(e);
                            }
                        }
                        if !self.eat(&TokenKind::Comma) {
                            break;
                        }
                    }
                    self.allow_struct_lit = prev;
                    let rp = self.expect(&TokenKind::RParen)?;
                    let span = expr.span.to(rp.span);
                    let id = self.node_id();
                    expr = Expr {
                        id,
                        kind: ExprKind::Call { callee: Box::new(expr), args },
                        span,
                    };
                }
                TokenKind::Dot => {
                    self.bump();
                    let field = self.expect_ident()?;
                    let span = expr.span.to(field.span);
                    let id = self.node_id();
                    expr =
                        Expr { id, kind: ExprKind::Field { base: Box::new(expr), field }, span };
                }
                TokenKind::LBracket => {
                    self.bump();
                    let prev = self.allow_struct_lit;
                    self.allow_struct_lit = true;
                    let index = self.parse_expr();
                    self.allow_struct_lit = prev;
                    let index = index?;
                    let rb = self.expect(&TokenKind::RBracket)?;
                    let span = expr.span.to(rb.span);
                    let id = self.node_id();
                    expr = Expr {
                        id,
                        kind: ExprKind::Index { base: Box::new(expr), index: Box::new(index) },
                        span,
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> PResult<Expr> {
        let sp = self.peek_span();
        let kind = match self.peek().clone() {
            TokenKind::Literal(LitValue::Int { value, suffix }) => {
                self.bump();
                ExprKind::Int { value, suffix }
            }
            TokenKind::Literal(LitValue::Float { value, suffix }) => {
                self.bump();
                ExprKind::Float { value, suffix }
            }
            TokenKind::Literal(LitValue::Str(s)) => {
                self.bump();
                ExprKind::Str(s)
            }
            TokenKind::Literal(LitValue::Char(c)) => {
                self.bump();
                ExprKind::Char(c)
            }
            TokenKind::Keyword(Keyword::Benar) => {
                self.bump();
                ExprKind::Bool(true)
            }
            TokenKind::Keyword(Keyword::Salah) => {
                self.bump();
                ExprKind::Bool(false)
            }
            TokenKind::Ident(name) => {
                self.bump();
                if self.allow_struct_lit && matches!(self.peek(), TokenKind::LBrace) {
                    return self.parse_struct_lit(Ident { name, span: sp });
                }
                ExprKind::Name(name)
            }
            TokenKind::LBracket => {
                return self.parse_array_lit();
            }
            TokenKind::LParen => {
                self.bump();
                let prev = self.allow_struct_lit;
                self.allow_struct_lit = true;
                let inner = self.parse_expr();
                self.allow_struct_lit = prev;
                let inner = inner?;
                self.expect(&TokenKind::RParen)?;

                return Ok(inner);
            }
            other => {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_UNEXPECTED_TOKEN,
                        format!("diharapkan ekspresi, ditemukan {}", other.describe()),
                    )
                    .primary(sp, "ekspresi tidak valid di sini"),
                );
                return Err(Bail);
            }
        };
        let id = self.node_id();
        Ok(Expr { id, kind, span: sp })
    }

    fn recover_to_item(&mut self) {
        while !self.at_eof() {
            if matches!(
                self.peek(),
                TokenKind::Keyword(
                    Keyword::Fungsi
                        | Keyword::Tipe
                        | Keyword::Enum
                        | Keyword::Utama
                        | Keyword::Ekstern
                        | Keyword::Gunakan
                        | Keyword::Impor
                        | Keyword::Publik
                )
            ) {
                return;
            }
            self.bump();
        }
    }

    fn recover_in_block(&mut self) {
        while !matches!(self.peek(), TokenKind::RBrace | TokenKind::Eof) {
            if matches!(self.peek(), TokenKind::Semicolon) {
                self.bump();
                return;
            }
            if matches!(
                self.peek(),
                TokenKind::Keyword(
                    Keyword::Nilai
                        | Keyword::Ubah
                        | Keyword::Jika
                        | Keyword::Selama
                        | Keyword::Untuk
                        | Keyword::Kembalikan
                        | Keyword::Berhenti
                        | Keyword::Lanjut
                )
            ) {
                return;
            }
            self.bump();
        }
    }
}

pub fn parse(file: FileId, tokens: Vec<Token>, diags: &mut DiagnosticEngine) -> Program {
    Parser::new(file, tokens, diags).parse_program()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lime_lexer::tokenize;
    use lime_span::SourceMap;

    fn parse_src(src: &str) -> (Program, DiagnosticEngine) {
        let mut sm = SourceMap::new();
        let f = sm.add("t.lime", src);
        let mut diags = DiagnosticEngine::new();
        let toks = tokenize(f, src, &mut diags);
        let prog = parse(f, toks, &mut diags);
        (prog, diags)
    }

    #[test]
    fn parses_hello() {
        let (p, d) = parse_src("program Halo\nutama {\n tampilkan(\"hai\")\n}");
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
        assert_eq!(p.name.name, "Halo");
        assert_eq!(p.items.len(), 1);
        assert!(matches!(p.items[0], Item::Main(_)));
    }

    #[test]
    fn parses_struct_literal() {
        let (p, d) = parse_src(
            "program P\nutama { nilai t = Titik { x: 1, y: 2 } }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
        let Item::Main(m) = &p.items[0] else { panic!() };
        let Stmt::Let { init, .. } = &m.body.stmts[0] else { panic!() };
        let ExprKind::StructLit { name, fields } = &init.kind else {
            panic!("expected struct literal, got {:?}", init.kind);
        };
        assert_eq!(name.name, "Titik");
        assert_eq!(fields.len(), 2);
    }

    #[test]
    fn struct_literal_not_parsed_in_if_condition() {
        let (p, d) = parse_src(
            "program P\nutama { jika x { tampilkan(\"y\") } }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
        let Item::Main(m) = &p.items[0] else { panic!() };
        let Stmt::If(i) = &m.body.stmts[0] else { panic!() };
        assert!(matches!(i.cond.kind, ExprKind::Name(_)), "cond should be a bare name");
    }

    #[test]
    fn parses_function_with_params_and_ret() {
        let (p, d) = parse_src(
            "program P\nfungsi tambah(a: angka, b: angka) -> angka {\n kembalikan a + b\n}",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
        match &p.items[0] {
            Item::Function(f) => {
                assert_eq!(f.name.name, "tambah");
                assert_eq!(f.params.len(), 2);
                assert!(matches!(
                    &f.ret.as_ref().unwrap().kind,
                    TypeExprKind::Named(n) if n == "angka"
                ));
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn precedence_is_correct() {

        let (p, d) = parse_src("program P\nfungsi f() -> angka { kembalikan 1 + 2 * 3 }");
        assert!(!d.has_errors());
        let Item::Function(f) = &p.items[0] else { panic!() };
        let Stmt::Return { value: Some(e), .. } = &f.body.stmts[0] else { panic!() };
        let ExprKind::Binary { op: BinOp::Add, rhs, .. } = &e.kind else {
            panic!("top op should be +");
        };
        assert!(matches!(rhs.kind, ExprKind::Binary { op: BinOp::Mul, .. }));
    }

    #[test]
    fn if_else_if_else_chain() {
        let (p, d) = parse_src(
            "program P\nutama { jika benar { } lain jika salah { } lain { } }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
        let Item::Main(m) = &p.items[0] else { panic!() };
        let Stmt::If(i) = &m.body.stmts[0] else { panic!() };
        assert_eq!(i.else_ifs.len(), 1);
        assert!(i.else_block.is_some());
    }

    #[test]
    fn for_loop_ranges() {
        let (p, d) = parse_src("program P\nutama { untuk i dari 0 sampai 10 { } }");
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
        let Item::Main(m) = &p.items[0] else { panic!() };
        let Stmt::For(f) = &m.body.stmts[0] else { panic!() };
        assert_eq!(f.var.name, "i");
        assert!(!f.inclusive);
    }

    #[test]
    fn missing_program_header_reports() {
        let (_p, d) = parse_src("utama { }");
        assert!(d.has_errors());
        assert!(d
            .diagnostics()
            .iter()
            .any(|x| x.code == Some(codes::E_MISSING_PROGRAM_HEADER)));
    }

    #[test]
    fn invalid_assign_target_reports() {
        let (_p, d) = parse_src("program P\nutama { 1 + 2 = 3 }");
        assert!(d.has_errors());
        assert!(d
            .diagnostics()
            .iter()
            .any(|x| x.code == Some(codes::E_INVALID_ASSIGN_TARGET)));
    }

    #[test]
    fn recovers_and_reports_multiple() {

        let (_p, d) = parse_src("program P\nutama { nilai = ; nilai y = 2 @ }");
        assert!(d.has_errors());
    }
}
