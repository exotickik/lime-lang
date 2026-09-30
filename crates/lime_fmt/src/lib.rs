use lime_ast::*;
use lime_diagnostics::DiagnosticEngine;
use lime_lexer::tokenize;
use lime_parser::parse;
use lime_span::SourceMap;

pub fn format_source(name: &str, src: &str) -> Result<String, Vec<String>> {
    let mut sm = SourceMap::new();
    let file = sm.add(name, src);
    let mut diags = DiagnosticEngine::new();
    let toks = tokenize(file, src, &mut diags);
    let program = parse(file, toks, &mut diags);
    if diags.has_errors() {
        let rendered: Vec<String> = diags
            .diagnostics()
            .iter()
            .map(|d| lime_diagnostics::render::render(d, &sm))
            .collect();
        return Err(rendered);
    }
    Ok(Printer::new().print_program(&program))
}

struct Printer {
    out: String,
    indent: usize,
}

impl Printer {
    fn new() -> Printer {
        Printer { out: String::new(), indent: 0 }
    }

    fn line(&mut self, text: &str) {
        for _ in 0..self.indent {
            self.out.push_str("    ");
        }
        self.out.push_str(text);
        self.out.push('\n');
    }

    fn blank(&mut self) {
        self.out.push('\n');
    }

    fn print_program(mut self, program: &Program) -> String {
        let kw = if program.is_module { "modul" } else { "program" };
        self.line(&format!("{} {}", kw, program.name.name));
        for item in &program.items {
            self.blank();
            self.print_item(item);
        }
        self.out
    }

    fn print_item(&mut self, item: &Item) {
        match item {
            Item::Function(f) => self.print_function(f),
            Item::Struct(s) => self.print_struct(s),
            Item::Enum(e) => self.print_enum(e),
            Item::Trait(t) => self.print_trait(t),
            Item::Impl(b) => self.print_impl(b),
            Item::Main(m) => {
                self.line("utama {");
                self.indent += 1;
                self.print_stmts(&m.body.stmts);
                self.indent -= 1;
                self.line("}");
            }
            Item::Extern(e) => self.print_extern(e),
            Item::Use(id) => self.line(&format!("gunakan {}", id.name)),
            Item::Import(id) => self.line(&format!("impor {}", id.name)),
        }
    }

    fn print_function(&mut self, f: &Function) {
        let vis = if f.public { "publik " } else { "" };
        let generics = if f.type_params.is_empty() {
            String::new()
        } else {
            let ps: Vec<String> = f
                .type_params
                .iter()
                .map(|p| {
                    if p.bounds.is_empty() {
                        p.name.name.clone()
                    } else {
                        let bs: Vec<String> = p.bounds.iter().map(|b| b.name.clone()).collect();
                        format!("{}: {}", p.name.name, bs.join(" + "))
                    }
                })
                .collect();
            format!("<{}>", ps.join(", "))
        };
        let params = self.params_str(&f.params);
        let ret = match &f.ret {
            Some(t) => format!(" -> {}", type_str(t)),
            None => String::new(),
        };
        self.line(&format!("{}fungsi {}{}({}){} {{", vis, f.name.name, generics, params, ret));
        self.indent += 1;
        self.print_stmts(&f.body.stmts);
        self.indent -= 1;
        self.line("}");
    }

    fn print_extern(&mut self, e: &ExternDecl) {
        let mut params = self.params_str(&e.params);
        if e.variadic {
            if params.is_empty() {
                params.push_str("...");
            } else {
                params.push_str(", ...");
            }
        }
        let ret = match &e.ret {
            Some(t) => format!(" -> {}", type_str(t)),
            None => String::new(),
        };
        self.line(&format!("ekstern fungsi {}({}){}", e.name.name, params, ret));
    }

    fn print_struct(&mut self, s: &StructDecl) {
        let vis = if s.public { "publik " } else { "" };
        self.line(&format!("{}tipe {} {{", vis, s.name.name));
        self.indent += 1;
        for field in &s.fields {
            let fvis = if field.public { "publik " } else { "" };
            self.line(&format!("{}{}: {}", fvis, field.name.name, type_str(&field.ty)));
        }
        self.indent -= 1;
        self.line("}");
    }

    fn print_enum(&mut self, e: &EnumDecl) {
        let vis = if e.public { "publik " } else { "" };
        self.line(&format!("{}enum {} {{", vis, e.name.name));
        self.indent += 1;
        for variant in &e.variants {
            self.line(&format!("{},", variant.name));
        }
        self.indent -= 1;
        self.line("}");
    }

    fn print_trait(&mut self, t: &TraitDecl) {
        let vis = if t.public { "publik " } else { "" };
        self.line(&format!("{}sifat {} {{", vis, t.name.name));
        self.indent += 1;
        for m in &t.methods {
            let ret = match &m.ret {
                Some(ty) => format!(" -> {}", type_str(ty)),
                None => String::new(),
            };
            self.line(&format!("fungsi {}({}){}", m.name.name, self.params_str(&m.params), ret));
        }
        self.indent -= 1;
        self.line("}");
    }

    fn print_impl(&mut self, b: &ImplBlock) {
        let head = match &b.trait_name {
            Some(t) => format!("implementasi {} untuk {} {{", t.name, b.type_name.name),
            None => format!("implementasi {} {{", b.type_name.name),
        };
        self.line(&head);
        self.indent += 1;
        for m in &b.methods {
            self.print_function(m);
        }
        self.indent -= 1;
        self.line("}");
    }

    fn params_str(&self, params: &[Param]) -> String {
        params
            .iter()
            .map(|p| format!("{}: {}", p.name.name, type_str(&p.ty)))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn print_stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.print_stmt(stmt);
        }
    }

    fn print_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { mutable, name, ty, init, .. } => {
                let kw = if *mutable { "ubah" } else { "nilai" };
                let annot = match ty {
                    Some(t) => format!(": {}", type_str(t)),
                    None => String::new(),
                };
                self.line(&format!("{} {}{} = {}", kw, name.name, annot, expr_str(init, 0)));
            }
            Stmt::Assign { target, value, .. } => {
                self.line(&format!("{} = {}", expr_str(target, 0), expr_str(value, 0)));
            }
            Stmt::Expr(e) => self.line(&expr_str(e, 0)),
            Stmt::Return { value, .. } => match value {
                Some(e) => self.line(&format!("kembalikan {}", expr_str(e, 0))),
                None => self.line("kembalikan"),
            },
            Stmt::Break(_) => self.line("berhenti"),
            Stmt::Continue(_) => self.line("lanjut"),
            Stmt::If(if_stmt) => self.print_if(if_stmt),
            Stmt::While { cond, body, .. } => {
                self.line(&format!("selama {} {{", expr_str(cond, 0)));
                self.indent += 1;
                self.print_stmts(&body.stmts);
                self.indent -= 1;
                self.line("}");
            }
            Stmt::For(f) => {
                let bound = if f.inclusive { "hingga" } else { "sampai" };
                self.line(&format!(
                    "untuk {} dari {} {} {} {{",
                    f.var.name,
                    expr_str(&f.start, 0),
                    bound,
                    expr_str(&f.end, 0)
                ));
                self.indent += 1;
                self.print_stmts(&f.body.stmts);
                self.indent -= 1;
                self.line("}");
            }
            Stmt::ForEach(f) => {
                self.line(&format!(
                    "untuk {} pada {} {{",
                    f.var.name,
                    expr_str(&f.iterable, 0)
                ));
                self.indent += 1;
                self.print_stmts(&f.body.stmts);
                self.indent -= 1;
                self.line("}");
            }
            Stmt::Block(b) => {
                self.line("{");
                self.indent += 1;
                self.print_stmts(&b.stmts);
                self.indent -= 1;
                self.line("}");
            }
        }
    }

    fn print_if(&mut self, if_stmt: &IfStmt) {
        self.line(&format!("jika {} {{", expr_str(&if_stmt.cond, 0)));
        self.indent += 1;
        self.print_stmts(&if_stmt.then_block.stmts);
        self.indent -= 1;
        for (cond, block) in &if_stmt.else_ifs {
            self.line(&format!("}} lain jika {} {{", expr_str(cond, 0)));
            self.indent += 1;
            self.print_stmts(&block.stmts);
            self.indent -= 1;
        }
        if let Some(block) = &if_stmt.else_block {
            self.line("} lain {");
            self.indent += 1;
            self.print_stmts(&block.stmts);
            self.indent -= 1;
        }
        self.line("}");
    }
}

fn type_str(t: &TypeExpr) -> String {
    match &t.kind {
        TypeExprKind::Named(name) => name.clone(),
        TypeExprKind::Array { elem, size } => format!("[{}; {}]", type_str(elem), size),
        TypeExprKind::Applied { name, args } => {
            let a: Vec<String> = args.iter().map(type_str).collect();
            format!("{}<{}>", name, a.join(", "))
        }
        TypeExprKind::Ref { inner, mutable } => {
            if *mutable {
                format!("&ubah {}", type_str(inner))
            } else {
                format!("&{}", type_str(inner))
            }
        }
        TypeExprKind::Fn { params, ret } => {
            let ps: Vec<String> = params.iter().map(type_str).collect();
            match ret {
                Some(r) => format!("fungsi({}) -> {}", ps.join(", "), type_str(r)),
                None => format!("fungsi({})", ps.join(", ")),
            }
        }
    }
}

fn bin_prec(op: BinOp) -> u8 {
    match op {
        BinOp::Or => 1,
        BinOp::And => 2,
        BinOp::Eq | BinOp::Ne => 3,
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => 4,
        BinOp::Add | BinOp::Sub => 5,
        BinOp::Mul | BinOp::Div | BinOp::Rem => 6,
    }
}

fn expr_str(expr: &Expr, parent_prec: u8) -> String {
    match &expr.kind {
        ExprKind::Int { value, suffix } => match suffix {
            Some(s) => format!("{}{}", value, s),
            None => value.to_string(),
        },
        ExprKind::Float { value, suffix } => match suffix {
            Some(s) => format!("{}{}", value, s),
            None => format_float(*value),
        },
        ExprKind::Str(s) => format!("\"{}\"", escape_str(s)),
        ExprKind::Char(c) => format!("'{}'", escape_char(*c)),
        ExprKind::Bool(b) => if *b { "benar".to_string() } else { "salah".to_string() },
        ExprKind::Name(n) => n.clone(),
        ExprKind::Binary { op, lhs, rhs } => {
            let prec = bin_prec(*op);
            let l = expr_str(lhs, prec);
            let r = expr_str(rhs, prec + 1);
            let s = format!("{} {} {}", l, op.symbol(), r);
            if prec < parent_prec {
                format!("({})", s)
            } else {
                s
            }
        }
        ExprKind::Unary { op, operand } => {
            let sym = match op {
                UnOp::Neg => "-",
                UnOp::Not => "!",
            };
            format!("{}{}", sym, expr_str(operand, 7))
        }
        ExprKind::Call { callee, args } => {
            let a: Vec<String> = args.iter().map(|e| expr_str(e, 0)).collect();
            format!("{}({})", expr_str(callee, 8), a.join(", "))
        }
        ExprKind::Field { base, field } => {
            format!("{}.{}", expr_str(base, 8), field.name)
        }
        ExprKind::StructLit { name, fields } => {
            if fields.is_empty() {
                format!("{} {{}}", name.name)
            } else {
                let fs: Vec<String> = fields
                    .iter()
                    .map(|f| format!("{}: {}", f.name.name, expr_str(&f.value, 0)))
                    .collect();
                format!("{} {{ {} }}", name.name, fs.join(", "))
            }
        }
        ExprKind::ArrayLit(elems) => {
            let es: Vec<String> = elems.iter().map(|e| expr_str(e, 0)).collect();
            format!("[{}]", es.join(", "))
        }
        ExprKind::Index { base, index } => {
            format!("{}[{}]", expr_str(base, 8), expr_str(index, 0))
        }
        ExprKind::Deref(inner) => format!("*{}", expr_str(inner, 7)),
        ExprKind::Try(inner) => format!("coba {}", expr_str(inner, 7)),
    }
}

fn format_float(v: f64) -> String {
    if v == v.trunc() && v.is_finite() {
        format!("{:.1}", v)
    } else {
        let s = format!("{}", v);
        s
    }
}

fn escape_str(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            _ => out.push(c),
        }
    }
    out
}

fn escape_char(c: char) -> String {
    match c {
        '\n' => "\\n".to_string(),
        '\r' => "\\r".to_string(),
        '\t' => "\\t".to_string(),
        '\\' => "\\\\".to_string(),
        '\'' => "\\'".to_string(),
        _ => c.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_and_is_idempotent() {
        let src = "program P\nfungsi f(a:angka,b:angka)->angka{kembalikan a+b*2}\nutama{tampilkan(f(1,2))}";
        let once = format_source("t.lime", src).unwrap();
        let twice = format_source("t.lime", &once).unwrap();
        assert_eq!(once, twice, "formatter must be idempotent");
        assert!(once.contains("fungsi f(a: angka, b: angka) -> angka {"));
        assert!(once.contains("    kembalikan a + b * 2"));
    }

    #[test]
    fn preserves_precedence_parens() {
        let src = "program P\nfungsi f() -> angka { kembalikan (1 + 2) * 3 }\nutama { }";
        let out = format_source("t.lime", src).unwrap();
        assert!(out.contains("(1 + 2) * 3"), "got: {out}");
    }

    #[test]
    fn errors_reported_for_invalid() {
        let res = format_source("t.lime", "program");
        assert!(res.is_err());
    }
}
