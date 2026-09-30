use crate::results::*;
use lime_ast::*;
use lime_diagnostics::{codes, Diagnostic, DiagnosticEngine};
use lime_span::Span;
use lime_types::{EnumId, StructId, Type};
use std::collections::{HashMap, HashSet};

pub struct Checker<'a> {
    diags: &'a mut DiagnosticEngine,
    results: TypeckResults,
    scopes: Vec<HashMap<String, Res>>,
    globals: HashMap<String, Res>,
    struct_ids: HashMap<String, StructId>,
    enum_ids: HashMap<String, EnumId>,
    generic_fns: Vec<Function>,
    instantiations: HashMap<(usize, Vec<Type>), usize>,
    mono_worklist: Vec<usize>,
    next_mono_node: u32,
    next_local: u32,
    current_ret: Type,
    current_self: Option<Type>,
    trait_ids: HashMap<String, usize>,
    loop_depth: u32,
    modules: crate::ModuleInfo,
    sym_module: HashMap<String, usize>,
    sym_public: HashMap<String, bool>,
    current_module: usize,
}

impl<'a> Checker<'a> {
    pub fn new(diags: &'a mut DiagnosticEngine) -> Checker<'a> {
        Checker {
            diags,
            results: TypeckResults::default(),
            scopes: Vec::new(),
            globals: HashMap::new(),
            struct_ids: HashMap::new(),
            enum_ids: HashMap::new(),
            generic_fns: Vec::new(),
            instantiations: HashMap::new(),
            mono_worklist: Vec::new(),
            next_mono_node: 1_000_000_000,
            next_local: 0,
            current_ret: Type::Unit,
            current_self: None,
            trait_ids: HashMap::new(),
            loop_depth: 0,
            modules: crate::ModuleInfo::default(),
            sym_module: HashMap::new(),
            sym_public: HashMap::new(),
            current_module: 0,
        }
    }

    pub fn set_modules(&mut self, info: crate::ModuleInfo) {
        self.modules = info;
    }

    fn module_of_span(&self, span: Span) -> usize {
        self.modules.file_module.get(&span.file).copied().unwrap_or(0)
    }

    fn accessible(&self, name: &str) -> bool {
        let Some(&m) = self.sym_module.get(name) else {
            return true;
        };
        if m == self.current_module {
            return true;
        }
        if !self.sym_public.get(name).copied().unwrap_or(false) {
            return false;
        }
        match self.modules.module_names.get(m) {
            Some(mod_name) => self
                .modules
                .imports
                .get(self.current_module)
                .map(|imps| imps.contains(mod_name))
                .unwrap_or(false),
            None => false,
        }
    }

    fn check_visibility(&mut self, name: &str, span: Span) {
        if self.modules.module_names.is_empty() || self.accessible(name) {
            return;
        }
        let owner = self
            .sym_module
            .get(name)
            .and_then(|&m| self.modules.module_names.get(m))
            .cloned()
            .unwrap_or_default();
        let public = self.sym_public.get(name).copied().unwrap_or(false);
        let msg = if public {
            format!("'{}' ada di modul '{}' yang belum di-'impor'", name, owner)
        } else {
            format!("'{}' bukan 'publik' di modul '{}'", name, owner)
        };
        self.diags.emit(
            Diagnostic::error(codes::E_VISIBILITY, msg)
                .primary(span, "tidak bisa diakses dari modul ini")
                .help("jadikan item 'publik' dan 'impor' modulnya"),
        );
    }

    fn record_symbols(&mut self, program: &Program) {
        for item in &program.items {
            let (name, span, public) = match item {
                Item::Function(f) => (f.name.name.clone(), f.name.span, f.public),
                Item::Struct(s) => (s.name.name.clone(), s.name.span, s.public),
                Item::Enum(e) => (e.name.name.clone(), e.name.span, e.public),
                Item::Trait(t) => (t.name.name.clone(), t.name.span, t.public),
                _ => continue,
            };
            let m = self.module_of_span(span);
            self.sym_module.insert(name.clone(), m);
            self.sym_public.insert(name, public);
        }
    }

    pub fn analyze(mut self, program: &Program) -> TypeckResults {
        self.collect_enums(program);
        self.collect_structs(program);
        self.resolve_struct_fields(program);
        self.collect_functions(program);
        self.collect_traits(program);
        self.collect_impls(program);
        self.record_symbols(program);
        self.register_globals();
        self.check_bodies(program);
        self.process_mono_worklist();
        self.check_main_presence(program);
        if !self.diags.has_errors() {
            let moved = crate::ownership::check_program(program, &self.results, self.diags);
            self.results.moved_uses = moved;
        }
        self.results
    }

    fn process_mono_worklist(&mut self) {
        while let Some(idx) = self.mono_worklist.pop() {
            let f = self.results.mono_functions[idx].clone();
            self.check_function(&f);
        }
    }

    fn collect_enums(&mut self, program: &Program) {
        for item in &program.items {
            if let Item::Enum(e) = item {
                if self.enum_ids.contains_key(&e.name.name)
                    || self.struct_ids.contains_key(&e.name.name)
                {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_REDEFINITION,
                            format!("tipe '{}' didefinisikan lebih dari sekali", e.name.name),
                        )
                        .primary(e.name.span, "definisi ulang di sini"),
                    );
                    continue;
                }
                let mut seen: HashMap<String, ()> = HashMap::new();
                let mut variants = Vec::new();
                for v in &e.variants {
                    if seen.insert(v.name.clone(), ()).is_some() {
                        self.diags.emit(
                            Diagnostic::error(
                                codes::E_REDEFINITION,
                                format!("varian '{}' didefinisikan lebih dari sekali", v.name),
                            )
                            .primary(v.span, "varian ganda"),
                        );
                        continue;
                    }
                    variants.push(v.name.clone());
                }
                let id = EnumId(self.results.enums.len() as u32);
                self.enum_ids.insert(e.name.name.clone(), id);
                self.results.enums.push(EnumInfo {
                    name: e.name.name.clone(),
                    variants,
                    def_span: e.name.span,
                });
            }
        }
    }

    fn collect_structs(&mut self, program: &Program) {
        for item in &program.items {
            if let Item::Struct(s) = item {
                if self.struct_ids.contains_key(&s.name.name) {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_REDEFINITION,
                            format!("tipe '{}' didefinisikan lebih dari sekali", s.name.name),
                        )
                        .primary(s.name.span, "definisi ulang di sini"),
                    );
                    continue;
                }
                let id = StructId(self.results.structs.len() as u32);
                self.struct_ids.insert(s.name.name.clone(), id);
                self.results.structs.push(StructInfo {
                    name: s.name.name.clone(),
                    fields: Vec::new(),
                    def_span: s.name.span,
                });
            }
        }
    }

    fn resolve_struct_fields(&mut self, program: &Program) {
        for item in &program.items {
            if let Item::Struct(s) = item {
                let Some(&id) = self.struct_ids.get(&s.name.name) else { continue };
                let mut fields = Vec::new();
                let mut seen = HashMap::new();
                for f in &s.fields {
                    if seen.insert(f.name.name.clone(), ()).is_some() {
                        self.diags.emit(
                            Diagnostic::error(
                                codes::E_REDEFINITION,
                                format!("field '{}' didefinisikan lebih dari sekali", f.name.name),
                            )
                            .primary(f.name.span, "field ganda"),
                        );
                    }
                    let ty = self.resolve_type(&f.ty);
                    fields.push(FieldInfo { name: f.name.name.clone(), ty, public: f.public });
                }
                self.results.structs[id.0 as usize].fields = fields;
            }
        }
    }

    fn collect_functions(&mut self, program: &Program) {
        for item in &program.items {
            match item {
                Item::Function(f) if !f.type_params.is_empty() => {
                    if is_builtin_name(&f.name.name)
                        || self.generic_fns.iter().any(|g| g.name.name == f.name.name)
                    {
                        self.diags.emit(
                            Diagnostic::error(
                                codes::E_REDEFINITION,
                                format!("fungsi '{}' didefinisikan lebih dari sekali", f.name.name),
                            )
                            .primary(f.name.span, "definisi ulang di sini"),
                        );
                        continue;
                    }
                    self.generic_fns.push(f.clone());
                }
                Item::Function(f) => {
                    let params = f
                        .params
                        .iter()
                        .map(|p| (p.name.name.clone(), self.resolve_type(&p.ty)))
                        .collect();
                    let ret = f.ret.as_ref().map(|t| self.resolve_type(t)).unwrap_or(Type::Unit);
                    self.push_fn(FnSig {
                        name: f.name.name.clone(),
                        params,
                        ret,
                        is_extern: false,
                        variadic: false,
                        public: f.public,
                        def_span: f.name.span,
                    });
                }
                Item::Extern(e) => {
                    let params = e
                        .params
                        .iter()
                        .map(|p| (p.name.name.clone(), self.resolve_type(&p.ty)))
                        .collect();
                    let ret = e.ret.as_ref().map(|t| self.resolve_type(t)).unwrap_or(Type::Unit);
                    self.push_fn(FnSig {
                        name: e.name.name.clone(),
                        params,
                        ret,
                        is_extern: true,
                        variadic: e.variadic,
                        public: true,
                        def_span: e.name.span,
                    });
                }
                _ => {}
            }
        }
    }

    fn push_fn(&mut self, sig: FnSig) {
        if is_builtin_name(&sig.name) {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_REDEFINITION,
                    format!("'{}' adalah nama builtin dan tidak bisa didefinisikan ulang", sig.name),
                )
                .primary(sig.def_span, "nama sudah dipakai builtin"),
            );
            return;
        }
        if self.results.func_by_name(&sig.name).is_some() {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_REDEFINITION,
                    format!("fungsi '{}' didefinisikan lebih dari sekali", sig.name),
                )
                .primary(sig.def_span, "definisi ulang di sini"),
            );
            return;
        }
        self.results.functions.push(sig);
    }

    fn collect_traits(&mut self, program: &Program) {
        for item in &program.items {
            if let Item::Trait(t) = item {
                if self.trait_ids.contains_key(&t.name.name) {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_REDEFINITION,
                            format!("sifat '{}' didefinisikan lebih dari sekali", t.name.name),
                        )
                        .primary(t.name.span, "definisi ulang di sini"),
                    );
                    continue;
                }
                self.current_self = Some(Type::Param("Diri".to_string()));
                let mut methods = Vec::new();
                let mut seen = HashSet::new();
                for m in &t.methods {
                    if !seen.insert(m.name.name.clone()) {
                        self.diags.emit(
                            Diagnostic::error(
                                codes::E_TRAIT_DUP_METHOD,
                                format!("metode '{}' ditulis dua kali di sifat '{}'", m.name.name, t.name.name),
                            )
                            .primary(m.name.span, "metode ganda"),
                        );
                        continue;
                    }
                    let params = m.params.iter().map(|p| self.resolve_type(&p.ty)).collect();
                    let ret = m.ret.as_ref().map(|r| self.resolve_type(r)).unwrap_or(Type::Unit);
                    methods.push(TraitMethodInfo { name: m.name.name.clone(), params, ret });
                }
                self.current_self = None;
                let idx = self.results.traits.len();
                self.trait_ids.insert(t.name.name.clone(), idx);
                self.results.traits.push(TraitInfo {
                    name: t.name.name.clone(),
                    methods,
                    def_span: t.span,
                });
            }
        }
    }

    fn collect_impls(&mut self, program: &Program) {
        for item in &program.items {
            let Item::Impl(b) = item else { continue };
            let Some(self_type) = self.resolve_impl_target(&b.type_name) else {
                continue;
            };
            let trait_name = b.trait_name.as_ref().map(|t| t.name.clone());
            if let Some(tn) = &b.trait_name {
                if !self.trait_ids.contains_key(&tn.name) {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_UNKNOWN_TRAIT_OR_TYPE,
                            format!("sifat '{}' tidak dikenal", tn.name),
                        )
                        .primary(tn.span, "bukan sifat yang dikenal"),
                    );
                    continue;
                }
            }
            if self
                .results
                .impls
                .iter()
                .any(|im| im.trait_name == trait_name && im.self_type == self_type)
            {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_DUP_IMPL,
                        "implementasi ganda untuk tipe ini",
                    )
                    .primary(b.span, "sudah ada implementasi yang sama"),
                );
                continue;
            }

            self.current_self = Some(self_type.clone());
            let mut method_map: HashMap<String, usize> = HashMap::new();
            for m in &b.methods {
                let mangled = mangle_method(&self_type, trait_name.as_deref(), &m.name.name);
                let params: Vec<(String, Type)> = m
                    .params
                    .iter()
                    .map(|p| (p.name.name.clone(), self.resolve_type(&p.ty)))
                    .collect();
                let ret = m.ret.as_ref().map(|r| self.resolve_type(r)).unwrap_or(Type::Unit);
                let fn_idx = self.results.functions.len();
                self.results.functions.push(FnSig {
                    name: mangled.clone(),
                    params,
                    ret,
                    is_extern: false,
                    variadic: false,
                    public: false,
                    def_span: m.name.span,
                });

                let mut inst = m.clone();
                inst.name.name = mangled;
                inst.type_params.clear();
                let subst: HashMap<String, Type> =
                    std::iter::once(("Diri".to_string(), self_type.clone())).collect();
                substitute_fn_types(&mut inst, &subst, &self.struct_ids, &self.enum_ids, &self.results);
                let mono_idx = self.results.mono_functions.len();
                self.results.mono_functions.push(inst);
                self.mono_worklist.push(mono_idx);

                method_map.insert(m.name.name.clone(), fn_idx);
            }
            self.current_self = None;

            if let Some(tn) = &b.trait_name {
                let tidx = self.trait_ids[&tn.name];
                self.check_trait_conformance(tidx, &self_type, &method_map, b);
            }

            self.results.impls.push(ImplInfo { trait_name, self_type, methods: method_map });
        }
    }

    fn resolve_impl_target(&mut self, name: &Ident) -> Option<Type> {
        if let Some(&id) = self.struct_ids.get(&name.name) {
            return Some(Type::Struct(id));
        }
        if let Some(&id) = self.enum_ids.get(&name.name) {
            return Some(Type::Enum(id));
        }
        if let Some(t) = Type::from_builtin_name(&name.name) {
            return Some(t);
        }
        self.diags.emit(
            Diagnostic::error(
                codes::E_UNKNOWN_TRAIT_OR_TYPE,
                format!("tipe '{}' tidak dikenal", name.name),
            )
            .primary(name.span, "bukan tipe yang bisa diimplementasi"),
        );
        None
    }

    fn check_trait_conformance(
        &mut self,
        trait_idx: usize,
        self_type: &Type,
        method_map: &HashMap<String, usize>,
        b: &ImplBlock,
    ) {
        let ti = self.results.traits[trait_idx].clone();
        let subst: HashMap<String, Type> =
            std::iter::once(("Diri".to_string(), self_type.clone())).collect();
        for tm in &ti.methods {
            let Some(&fn_idx) = method_map.get(&tm.name) else {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_IMPL_METHOD_MISMATCH,
                        format!("implementasi '{}' belum punya metode '{}'", ti.name, tm.name),
                    )
                    .primary(b.span, "metode dari sifat belum diisi"),
                );
                continue;
            };
            let sig = self.results.functions[fn_idx].clone();
            let want_params: Vec<Type> =
                tm.params.iter().map(|p| subst_type(p, &subst)).collect();
            let want_ret = subst_type(&tm.ret, &subst);
            let got_params: Vec<Type> = sig.params.iter().map(|(_, t)| t.clone()).collect();
            if want_params != got_params || want_ret != sig.ret {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_IMPL_METHOD_MISMATCH,
                        format!("tanda tangan metode '{}' tidak cocok dengan sifat '{}'", tm.name, ti.name),
                    )
                    .primary(sig.def_span, "tanda tangan berbeda dari sifat"),
                );
            }
        }
        for name in method_map.keys() {
            if ti.method(name).is_none() {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_IMPL_METHOD_MISMATCH,
                        format!("metode '{}' bukan bagian dari sifat '{}'", name, ti.name),
                    )
                    .primary(b.span, "metode ini tidak ada di sifat"),
                );
            }
        }
    }

    fn type_implements(&self, ty: &Type, trait_name: &str) -> bool {
        self.results
            .impls
            .iter()
            .any(|im| im.trait_name.as_deref() == Some(trait_name) && im.self_type == *ty)
    }

    fn resolve_method(&mut self, self_ty: &Type, field: &Ident, call: &Expr) -> Option<(usize, RecvMode)> {
        let mut found: Vec<usize> = Vec::new();
        for im in &self.results.impls {
            if im.self_type == *self_ty {
                if let Some(&idx) = im.methods.get(&field.name) {
                    found.push(idx);
                }
            }
        }
        if found.is_empty() {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_METHOD_NOT_FOUND,
                    format!("tipe '{}' tidak punya metode '{}'", self_ty.display(), field.name),
                )
                .primary(field.span, "metode tidak ditemukan")
                .secondary(call.span, "pada pemanggilan ini"),
            );
            return None;
        }
        if found.len() > 1 {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_METHOD_AMBIGUOUS,
                    format!("metode '{}' ada di lebih dari satu sifat untuk tipe ini", field.name),
                )
                .primary(field.span, "metode ambigu"),
            );
            return None;
        }
        let fn_idx = found[0];
        let sig = &self.results.functions[fn_idx];
        let mode = match sig.params.first() {
            Some((_, Type::Ref { mutable: true, .. })) => RecvMode::RefMut,
            Some((_, Type::Ref { mutable: false, .. })) => RecvMode::Ref,
            Some(_) => RecvMode::Value,
            None => {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_METHOD_NOT_FOUND,
                        format!("metode '{}' tidak punya penerima 'diri'", field.name),
                    )
                    .primary(field.span, "metode tanpa penerima"),
                );
                return None;
            }
        };
        Some((fn_idx, mode))
    }

    fn check_method_call(&mut self, call: &Expr, base: &Expr, field: &Ident, args: &[Expr]) -> Type {
        let recv_ty = self.check_expr(base, None);
        if recv_ty.is_error() {
            for a in args {
                self.check_expr(a, None);
            }
            return Type::Error;
        }
        let lookup_ty = match &recv_ty {
            Type::Ref { inner, .. } => (**inner).clone(),
            t => t.clone(),
        };
        let Some((fn_idx, mode)) = self.resolve_method(&lookup_ty, field, call) else {
            for a in args {
                self.check_expr(a, None);
            }
            return Type::Error;
        };
        self.results
            .method_calls
            .insert(call.id, MethodResolution { func_idx: fn_idx, recv_mode: mode });

        if mode == RecvMode::RefMut {
            let ok_place = base.is_lvalue() && self.place_is_mutable(base);
            let ok_mut_ref = matches!(&recv_ty, Type::Ref { mutable: true, .. });
            if !ok_place && !ok_mut_ref {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_BORROW_IMMUTABLE,
                        format!("metode '{}' butuh penerima yang bisa diubah ('&ubah Diri')", field.name),
                    )
                    .primary(base.span, "penerima ini tidak bisa diubah")
                    .help("deklarasikan penerima dengan 'ubah'"),
                );
            }
        }

        let sig = self.results.functions[fn_idx].clone();
        let want = sig.params.len().saturating_sub(1);
        if args.len() != want {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_ARITY_MISMATCH,
                    format!("metode '{}' butuh {} argumen, diberi {}", field.name, want, args.len()),
                )
                .primary(call.span, "jumlah argumen salah"),
            );
            for a in args {
                self.check_expr(a, None);
            }
            return sig.ret;
        }
        for (a, (_, pty)) in args.iter().zip(sig.params.iter().skip(1)) {
            let at = self.check_expr(a, Some(pty));
            if !at.assignable_to(pty) {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_TYPE_MISMATCH_ASSIGN,
                        format!("argumen bertipe '{}', diharapkan '{}'", at.display(), pty.display()),
                    )
                    .primary(a.span, format!("bertipe '{}'", at.display())),
                );
            }
        }
        sig.ret
    }

    fn register_globals(&mut self) {
        for (idx, f) in self.results.functions.iter().enumerate() {
            self.globals.insert(f.name.clone(), Res::Func(idx));
        }
        for (gidx, f) in self.generic_fns.iter().enumerate() {
            self.globals.insert(f.name.name.clone(), Res::GenericFn(gidx));
        }
        for (name, id) in &self.struct_ids {
            self.globals.insert(name.clone(), Res::Struct(*id));
        }
        for (name, id) in &self.enum_ids {
            self.globals.insert(name.clone(), Res::Enum(*id));
        }
        self.globals.insert("tampilkan".to_string(), Res::Builtin(Builtin::Tampilkan));
        self.globals.insert("panjang".to_string(), Res::Builtin(Builtin::Panjang));
        self.globals.insert("kotak".to_string(), Res::Builtin(Builtin::Kotak));
        self.globals.insert("isi".to_string(), Res::Builtin(Builtin::Isi));
        self.globals.insert("ambil".to_string(), Res::Builtin(Builtin::Ambil));
        self.globals.insert("ambil_ubah".to_string(), Res::Builtin(Builtin::AmbilUbah));
        self.globals.insert("sukses".to_string(), Res::Builtin(Builtin::Sukses));
        self.globals.insert("gagal".to_string(), Res::Builtin(Builtin::Gagal));
        self.globals.insert("berhasil".to_string(), Res::Builtin(Builtin::Berhasil));
        self.globals.insert("nilai_atau".to_string(), Res::Builtin(Builtin::NilaiAtau));
        self.globals.insert("gagal_atau".to_string(), Res::Builtin(Builtin::GagalAtau));
    }

    fn check_bodies(&mut self, program: &Program) {
        for item in &program.items {
            match item {
                Item::Function(f) if f.type_params.is_empty() => self.check_function(f),
                Item::Main(m) => self.check_main(m),
                _ => {}
            }
        }
    }

    fn check_function(&mut self, f: &Function) {
        let sig = self.results.func_by_name(&f.name.name).map(|(_, s)| s.clone());
        let Some(sig) = sig else { return };
        self.current_ret = sig.ret.clone();
        self.current_module = self.module_of_span(f.name.span);
        self.loop_depth = 0;
        self.enter_scope();
        for (p, (pname, pty)) in f.params.iter().zip(sig.params.iter()) {
            let id = self.alloc_local(pname.clone(), pty.clone(), false);
            self.declare(&p.name.name, Res::Local(id), p.name.span);
            self.results.local_of_binding.insert(p.id, id);
        }
        self.check_block(&f.body);
        self.exit_scope();

        if sig.ret != Type::Unit && !block_always_returns(&f.body) {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_MISSING_RETURN,
                    format!(
                        "fungsi '{}' tidak mengembalikan '{}' pada semua jalur",
                        f.name.name,
                        sig.ret.display()
                    ),
                )
                .primary(f.name.span, "sebagian jalur tidak memiliki 'kembalikan'")
                .help("tambahkan 'kembalikan <nilai>' di akhir setiap cabang"),
            );
        }
    }

    fn check_main(&mut self, m: &MainBlock) {
        self.current_ret = Type::I64;
        self.current_module = self.module_of_span(m.span);
        self.loop_depth = 0;
        self.enter_scope();
        self.check_block(&m.body);
        self.exit_scope();
    }

    fn check_main_presence(&mut self, program: &Program) {
        let mains: Vec<&MainBlock> = program
            .items
            .iter()
            .filter_map(|i| if let Item::Main(m) = i { Some(m) } else { None })
            .collect();
        self.results.main_present = !mains.is_empty();
        if mains.is_empty() {
            self.diags.emit(
                Diagnostic::error(codes::E_MISSING_MAIN, "program tidak memiliki blok 'utama'")
                    .primary(program.header_span, "tambahkan blok 'utama { ... }'"),
            );
        } else if mains.len() > 1 {
            for extra in &mains[1..] {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_DUPLICATE_MAIN,
                        "hanya boleh ada satu blok 'utama'",
                    )
                    .primary(extra.span, "blok 'utama' tambahan"),
                );
            }
        }
    }

    fn check_block(&mut self, block: &Block) {
        self.enter_scope();
        let mut terminated: Option<Span> = None;
        for stmt in &block.stmts {
            if let Some(_prev) = terminated {
                self.diags.emit(
                    Diagnostic::warning(codes::W_UNREACHABLE, "kode ini tidak akan dieksekusi")
                        .primary(stmt_span(stmt), "tidak terjangkau"),
                );
                terminated = None;
            }
            self.check_stmt(stmt);
            if stmt_is_terminator(stmt) {
                terminated = Some(stmt_span(stmt));
            }
        }
        self.exit_scope();
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { mutable, name, ty, init, id: let_id, .. } => {
                let annotated = ty.as_ref().map(|t| self.resolve_type(t));
                let init_ty = self.check_expr(init, annotated.as_ref());
                let decl_ty = annotated.clone().unwrap_or_else(|| init_ty.clone());
                if let Some(ann) = &annotated {
                    if !init_ty.assignable_to(ann) {
                        self.diags.emit(
                            Diagnostic::error(
                                codes::E_TYPE_MISMATCH_ASSIGN,
                                format!(
                                    "tipe nilai awal '{}' tidak cocok dengan '{}'",
                                    init_ty.display(),
                                    ann.display()
                                ),
                            )
                            .primary(init.span, format!("bertipe '{}'", init_ty.display()))
                            .secondary(name.span, format!("dideklarasikan '{}'", ann.display())),
                        );
                    }
                }
                let id = self.alloc_local(name.name.clone(), decl_ty, *mutable);
                self.declare(&name.name, Res::Local(id), name.span);
                self.results.local_of_binding.insert(*let_id, id);
            }
            Stmt::Assign { target, value, .. } => {
                let target_ty = self.check_expr(target, None);
                self.check_assignable(target);
                let value_ty = self.check_expr(value, Some(&target_ty));
                if !value_ty.assignable_to(&target_ty) {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_TYPE_MISMATCH_ASSIGN,
                            format!(
                                "tidak bisa menetapkan '{}' ke target '{}'",
                                value_ty.display(),
                                target_ty.display()
                            ),
                        )
                        .primary(value.span, format!("bertipe '{}'", value_ty.display())),
                    );
                }
            }
            Stmt::Expr(e) => {
                self.check_expr(e, None);
            }
            Stmt::If(if_stmt) => self.check_if(if_stmt),
            Stmt::While { cond, body, .. } => {
                let ct = self.check_expr(cond, Some(&Type::Bool));
                self.expect_bool(&ct, cond.span, "kondisi 'selama'");
                self.loop_depth += 1;
                self.check_block(body);
                self.loop_depth -= 1;
            }
            Stmt::For(for_stmt) => self.check_for(for_stmt),
            Stmt::ForEach(fe) => self.check_foreach(fe),
            Stmt::Return { value, span } => {
                let ret = self.current_ret.clone();
                match value {
                    Some(e) => {
                        let t = self.check_expr(e, Some(&ret));
                        if !t.assignable_to(&ret) {
                            self.diags.emit(
                                Diagnostic::error(
                                    codes::E_RETURN_TYPE_MISMATCH,
                                    format!(
                                        "mengembalikan '{}' tetapi fungsi bertipe '{}'",
                                        t.display(),
                                        ret.display()
                                    ),
                                )
                                .primary(e.span, format!("bertipe '{}'", t.display())),
                            );
                        }
                    }
                    None => {
                        if ret != Type::Unit && ret != Type::I64 {
                            self.diags.emit(
                                Diagnostic::error(
                                    codes::E_RETURN_TYPE_MISMATCH,
                                    format!(
                                        "'kembalikan' tanpa nilai tetapi fungsi bertipe '{}'",
                                        ret.display()
                                    ),
                                )
                                .primary(*span, "nilai kembalian hilang"),
                            );
                        }
                    }
                }
            }
            Stmt::Break(sp) => {
                if self.loop_depth == 0 {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_LOOP_CTRL_OUTSIDE_LOOP,
                            "'berhenti' hanya boleh di dalam perulangan",
                        )
                        .primary(*sp, "di luar perulangan"),
                    );
                }
            }
            Stmt::Continue(sp) => {
                if self.loop_depth == 0 {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_LOOP_CTRL_OUTSIDE_LOOP,
                            "'lanjut' hanya boleh di dalam perulangan",
                        )
                        .primary(*sp, "di luar perulangan"),
                    );
                }
            }
            Stmt::Block(b) => self.check_block(b),
        }
    }

    fn check_if(&mut self, if_stmt: &IfStmt) {
        let ct = self.check_expr(&if_stmt.cond, Some(&Type::Bool));
        self.expect_bool(&ct, if_stmt.cond.span, "kondisi 'jika'");
        self.check_block(&if_stmt.then_block);
        for (cond, block) in &if_stmt.else_ifs {
            let t = self.check_expr(cond, Some(&Type::Bool));
            self.expect_bool(&t, cond.span, "kondisi 'lain jika'");
            self.check_block(block);
        }
        if let Some(block) = &if_stmt.else_block {
            self.check_block(block);
        }
    }

    fn check_for(&mut self, for_stmt: &ForStmt) {
        let start_ty = self.check_expr(&for_stmt.start, Some(&Type::I64));
        let var_ty = if start_ty.is_integer() {
            start_ty.clone()
        } else {
            if !start_ty.is_error() {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_INVALID_OPERAND,
                        "batas awal perulangan harus bilangan bulat",
                    )
                    .primary(for_stmt.start.span, format!("bertipe '{}'", start_ty.display())),
                );
            }
            Type::I64
        };
        let end_ty = self.check_expr(&for_stmt.end, Some(&var_ty));
        if !end_ty.assignable_to(&var_ty) && !end_ty.is_error() {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_TYPE_MISMATCH_BINOP,
                    format!(
                        "batas rentang bertipe berbeda: '{}' dan '{}'",
                        var_ty.display(),
                        end_ty.display()
                    ),
                )
                .primary(for_stmt.end.span, format!("bertipe '{}'", end_ty.display())),
            );
        }
        self.enter_scope();
        let id = self.alloc_local(for_stmt.var.name.clone(), var_ty, false);
        self.declare(&for_stmt.var.name, Res::Local(id), for_stmt.var.span);
        self.results.local_of_binding.insert(for_stmt.var_id, id);
        self.loop_depth += 1;
        self.check_block(&for_stmt.body);
        self.loop_depth -= 1;
        self.exit_scope();
    }

    fn check_foreach(&mut self, fe: &ForEachStmt) {
        let iter_ty = self.check_expr(&fe.iterable, None);
        let elem_ty = match &iter_ty {
            Type::Array { elem, .. } => (**elem).clone(),
            Type::Error => Type::Error,
            other => {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_NOT_ITERABLE,
                        format!("nilai bertipe '{}' tidak bisa diiterasi", other.display()),
                    )
                    .primary(fe.iterable.span, "hanya larik yang bisa diiterasi dengan 'pada'"),
                );
                Type::Error
            }
        };
        self.enter_scope();
        let id = self.alloc_local(fe.var.name.clone(), elem_ty, false);
        self.declare(&fe.var.name, Res::Local(id), fe.var.span);
        self.results.local_of_binding.insert(fe.var_id, id);
        self.loop_depth += 1;
        self.check_block(&fe.body);
        self.loop_depth -= 1;
        self.exit_scope();
    }

    fn check_expr(&mut self, expr: &Expr, expected: Option<&Type>) -> Type {
        let ty = self.check_expr_inner(expr, expected);
        self.results.expr_types.insert(expr.id, ty.clone());
        ty
    }

    fn check_expr_inner(&mut self, expr: &Expr, expected: Option<&Type>) -> Type {
        match &expr.kind {
            ExprKind::Int { suffix, .. } => {
                if let Some(s) = suffix {
                    Type::from_builtin_name(s).unwrap_or(Type::I64)
                } else if let Some(exp) = expected {
                    if exp.is_numeric() {
                        exp.clone()
                    } else {
                        Type::I64
                    }
                } else {
                    Type::I64
                }
            }
            ExprKind::Float { suffix, .. } => {
                if let Some(s) = suffix {
                    Type::from_builtin_name(s).unwrap_or(Type::F64)
                } else if let Some(exp) = expected {
                    if exp.is_float() {
                        exp.clone()
                    } else {
                        Type::F64
                    }
                } else {
                    Type::F64
                }
            }
            ExprKind::Str(_) => Type::Str,
            ExprKind::Char(_) => Type::Char,
            ExprKind::Bool(_) => Type::Bool,
            ExprKind::Name(name) => match self.lookup(name) {
                Some(res) => {
                    self.results.name_res.insert(expr.id, res.clone());
                    if !matches!(res, Res::Local(_) | Res::Builtin(_)) {
                        self.check_visibility(name, expr.span);
                    }
                    match res {
                        Res::Local(id) => self.results.locals[&id].ty.clone(),
                        Res::Func(idx) => {
                            self.results.func_values.insert(expr.id);
                            self.results.functions[idx].fn_type()
                        }
                        Res::Builtin(_) => Type::Error,
                        Res::Struct(_) => Type::Error,
                        Res::Enum(_) => Type::Error,
                        Res::GenericFn(_) => Type::Error,
                    }
                }
                None => {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_UNDEFINED_NAME,
                            format!("nama '{}' belum didefinisikan", name),
                        )
                        .primary(expr.span, "tidak ditemukan pada scope ini"),
                    );
                    Type::Error
                }
            },
            ExprKind::Binary { op, lhs, rhs } => self.check_binary(*op, lhs, rhs, expected),
            ExprKind::Unary { op, operand } => self.check_unary(*op, operand),
            ExprKind::Call { callee, args } => self.check_call(expr, callee, args, expected),
            ExprKind::Field { base, field } => {
                if let ExprKind::Name(n) = &base.kind {
                    if let Some(&eid) = self.enum_ids.get(n) {
                        return self.check_enum_variant(expr.id, eid, field);
                    }
                }
                self.check_field(base, field)
            }
            ExprKind::StructLit { name, fields } => self.check_struct_lit(name, fields),
            ExprKind::ArrayLit(elems) => self.check_array_lit(elems, expected),
            ExprKind::Index { base, index } => self.check_index(base, index),
            ExprKind::Deref(inner) => self.check_deref(inner),
            ExprKind::Try(inner) => self.check_try(inner),
        }
    }

    fn check_array_lit(&mut self, elems: &[Expr], expected: Option<&Type>) -> Type {
        let expected_elem = match expected {
            Some(Type::Array { elem, .. }) => Some((**elem).clone()),
            _ => None,
        };
        if elems.is_empty() {
            match expected {
                Some(Type::Array { elem, .. }) => {
                    return Type::Array { elem: elem.clone(), size: 0 };
                }
                _ => {
                    return Type::Array { elem: Box::new(Type::Error), size: 0 };
                }
            }
        }
        let first = self.check_expr(&elems[0], expected_elem.as_ref());
        let elem_ty = if first.is_error() {
            expected_elem.clone().unwrap_or(Type::Error)
        } else {
            first
        };
        for e in &elems[1..] {
            let t = self.check_expr(e, Some(&elem_ty));
            if !t.assignable_to(&elem_ty) {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_TYPE_MISMATCH_ASSIGN,
                        format!(
                            "elemen larik bertipe '{}', diharapkan '{}'",
                            t.display(),
                            elem_ty.display()
                        ),
                    )
                    .primary(e.span, format!("bertipe '{}'", t.display())),
                );
            }
        }
        Type::Array { elem: Box::new(elem_ty), size: elems.len() as u64 }
    }

    fn check_index(&mut self, base: &Expr, index: &Expr) -> Type {
        let base_ty = self.check_expr(base, None);
        let idx_ty = self.check_expr(index, Some(&Type::I64));
        if !idx_ty.is_error() && !idx_ty.is_integer() {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_INVALID_OPERAND,
                    format!("indeks larik harus bilangan bulat, bukan '{}'", idx_ty.display()),
                )
                .primary(index.span, "indeks tidak valid"),
            );
        }
        match base_ty {
            Type::Array { elem, .. } => (*elem).clone(),
            Type::Error => Type::Error,
            other => {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_NOT_INDEXABLE,
                        format!("nilai bertipe '{}' tidak bisa diindeks", other.display()),
                    )
                    .primary(base.span, "hanya larik yang bisa diindeks"),
                );
                Type::Error
            }
        }
    }

    fn check_struct_lit(&mut self, name: &Ident, fields: &[StructLitField]) -> Type {
        let id = match self.struct_ids.get(&name.name) {
            Some(&id) => {
                self.check_visibility(&name.name, name.span);
                id
            }
            None => {
                let msg = if Type::from_builtin_name(&name.name).is_some() {
                    format!("tipe bawaan '{}' tidak bisa dibangun dengan '{{ }}'", name.name)
                } else {
                    format!("'{}' bukan tipe struct", name.name)
                };
                self.diags.emit(
                    Diagnostic::error(codes::E_NOT_A_STRUCT, msg)
                        .primary(name.span, "bukan struct yang dikenal"),
                );
                for f in fields {
                    self.check_expr(&f.value, None);
                }
                return Type::Error;
            }
        };

        let info = self.results.struct_info(id).clone();
        let mut seen: HashMap<String, Span> = HashMap::new();

        for f in fields {
            if let Some(prev) = seen.insert(f.name.name.clone(), f.name.span) {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_DUPLICATE_FIELD,
                        format!("field '{}' diisi lebih dari sekali", f.name.name),
                    )
                    .primary(f.name.span, "pengisian ganda")
                    .secondary(prev, "pertama kali di sini"),
                );
            }
            match info.field(&f.name.name) {
                Some((_, field_info)) => {
                    let vty = self.check_expr(&f.value, Some(&field_info.ty));
                    if !vty.assignable_to(&field_info.ty) {
                        self.diags.emit(
                            Diagnostic::error(
                                codes::E_TYPE_MISMATCH_ASSIGN,
                                format!(
                                    "field '{}' bertipe '{}', diberi '{}'",
                                    f.name.name,
                                    field_info.ty.display(),
                                    vty.display()
                                ),
                            )
                            .primary(f.value.span, format!("bertipe '{}'", vty.display())),
                        );
                    }
                }
                None => {
                    self.check_expr(&f.value, None);
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_UNKNOWN_FIELD,
                            format!("tipe '{}' tidak memiliki field '{}'", info.name, f.name.name),
                        )
                        .primary(f.name.span, "field tidak dikenal"),
                    );
                }
            }
        }

        let missing: Vec<String> = info
            .fields
            .iter()
            .filter(|fi| !seen.contains_key(&fi.name))
            .map(|fi| fi.name.clone())
            .collect();
        if !missing.is_empty() {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_MISSING_FIELD,
                    format!("field belum diisi: {}", missing.join(", ")),
                )
                .primary(name.span, format!("struct '{}' butuh semua field", info.name)),
            );
        }

        Type::Struct(id)
    }

    fn check_binary(&mut self, op: BinOp, lhs: &Expr, rhs: &Expr, expected: Option<&Type>) -> Type {
        if op.is_logical() {
            let lt = self.check_expr(lhs, Some(&Type::Bool));
            let rt = self.check_expr(rhs, Some(&Type::Bool));
            self.expect_bool(&lt, lhs.span, "operand logika");
            self.expect_bool(&rt, rhs.span, "operand logika");
            return Type::Bool;
        }

        let lt = self.check_expr(lhs, if op.is_arithmetic() { expected } else { None });
        let rhs_expected = if lt.is_error() { None } else { Some(lt.clone()) };
        let rt = self.check_expr(rhs, rhs_expected.as_ref());

        if lt.is_error() || rt.is_error() {
            return if op.is_comparison() { Type::Bool } else { Type::Error };
        }

        if lt != rt {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_TYPE_MISMATCH_BINOP,
                    format!(
                        "operand '{}' tidak cocok: '{}' dan '{}'",
                        op.symbol(),
                        lt.display(),
                        rt.display()
                    ),
                )
                .primary(lhs.span.to(rhs.span), "tipe kedua sisi harus sama")
                .help("Lime tidak melakukan konversi numerik implisit"),
            );
            return if op.is_comparison() { Type::Bool } else { Type::Error };
        }

        if op.is_comparison() {
            if op == BinOp::Eq || op == BinOp::Ne {
                if !matches!(
                    lt,
                    Type::Int { .. }
                        | Type::Float(_)
                        | Type::Bool
                        | Type::Char
                        | Type::Str
                        | Type::Enum(_)
                ) {
                    self.invalid_operand(op, lhs.span.to(rhs.span), &lt);
                    return Type::Bool;
                }
            } else if !lt.is_numeric() {
                self.invalid_operand(op, lhs.span.to(rhs.span), &lt);
            }
            return Type::Bool;
        }

        if !lt.is_numeric() {
            self.invalid_operand(op, lhs.span.to(rhs.span), &lt);
            return Type::Error;
        }
        lt
    }

    fn check_unary(&mut self, op: UnOp, operand: &Expr) -> Type {
        match op {
            UnOp::Neg => {
                let t = self.check_expr(operand, None);
                if t.is_error() {
                    return Type::Error;
                }
                if !t.is_numeric() {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_INVALID_OPERAND,
                            format!("negasi '-' butuh tipe numerik, bukan '{}'", t.display()),
                        )
                        .primary(operand.span, "operand tidak numerik"),
                    );
                    return Type::Error;
                }
                t
            }
            UnOp::Not => {
                let t = self.check_expr(operand, Some(&Type::Bool));
                self.expect_bool(&t, operand.span, "operand 'bukan'");
                Type::Bool
            }
        }
    }

    fn check_call(&mut self, call: &Expr, callee: &Expr, args: &[Expr], expected: Option<&Type>) -> Type {
        if let ExprKind::Field { base, field } = &callee.kind {
            let enum_path = matches!(&base.kind, ExprKind::Name(n) if self.enum_ids.contains_key(n));
            if !enum_path {
                return self.check_method_call(call, base, field, args);
            }
        }
        if let ExprKind::Name(name) = &callee.kind {
            if let Some(Res::Builtin(b)) = self.lookup(name) {
                if matches!(
                    b,
                    Builtin::Sukses
                        | Builtin::Gagal
                        | Builtin::Berhasil
                        | Builtin::NilaiAtau
                        | Builtin::GagalAtau
                ) {
                    self.results.name_res.insert(callee.id, Res::Builtin(b));
                    return self.check_hasil_builtin(b, call, args, expected);
                }
            }
        }
        let ExprKind::Name(name) = &callee.kind else {
            return self.check_indirect_call(call, callee, args);
        };

        if matches!(self.lookup(name), Some(Res::Func(_)) | Some(Res::GenericFn(_))) {
            self.check_visibility(name, callee.span);
        }
        match self.lookup(name) {
            Some(Res::GenericFn(gidx)) => self.check_generic_call(gidx, call, callee, args),
            Some(Res::Builtin(Builtin::Kotak)) => {
                self.results.name_res.insert(callee.id, Res::Builtin(Builtin::Kotak));
                if args.len() != 1 {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_ARITY_MISMATCH,
                            format!("'kotak' butuh 1 argumen, diberi {}", args.len()),
                        )
                        .primary(call.span, "jumlah argumen salah"),
                    );
                    for a in args {
                        self.check_expr(a, None);
                    }
                    return Type::Error;
                }
                let at = self.check_expr(&args[0], None);
                if at.is_error() {
                    return Type::Error;
                }
                Type::Kotak(Box::new(at))
            }
            Some(Res::Builtin(Builtin::Isi)) => {
                self.results.name_res.insert(callee.id, Res::Builtin(Builtin::Isi));
                if args.len() != 1 {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_ARITY_MISMATCH,
                            format!("'isi' butuh 1 argumen, diberi {}", args.len()),
                        )
                        .primary(call.span, "jumlah argumen salah"),
                    );
                    for a in args {
                        self.check_expr(a, None);
                    }
                    return Type::Error;
                }
                let at = self.check_expr(&args[0], None);
                let boxed = match &at {
                    Type::Kotak(inner) => Some((**inner).clone()),
                    Type::Ref { inner, .. } => match &**inner {
                        Type::Kotak(k) => Some((**k).clone()),
                        _ => None,
                    },
                    _ => None,
                };
                match boxed {
                    Some(inner) => {
                        if !self.is_copy(&inner) {
                            self.diags.emit(
                                Diagnostic::error(
                                    codes::E_INVALID_OPERAND,
                                    format!(
                                        "'isi' hanya untuk kotak berisi tipe salinan, bukan '{}'",
                                        inner.display()
                                    ),
                                )
                                .primary(args[0].span, "isi kotak bertipe milik")
                                .help("nilai bertipe milik harus dipindahkan, bukan disalin keluar"),
                            );
                        }
                        inner
                    }
                    None if at.is_error() => Type::Error,
                    None => {
                        self.diags.emit(
                            Diagnostic::error(
                                codes::E_INVALID_OPERAND,
                                format!("'isi' butuh 'kotak<T>' atau reference-nya, bukan '{}'", at.display()),
                            )
                            .primary(args[0].span, "bukan kotak"),
                        );
                        Type::Error
                    }
                }
            }
            Some(Res::Builtin(Builtin::Ambil)) => {
                self.results.name_res.insert(callee.id, Res::Builtin(Builtin::Ambil));
                self.check_borrow(call, args, false)
            }
            Some(Res::Builtin(Builtin::AmbilUbah)) => {
                self.results.name_res.insert(callee.id, Res::Builtin(Builtin::AmbilUbah));
                self.check_borrow(call, args, true)
            }
            Some(Res::Builtin(Builtin::Panjang)) => {
                self.results.name_res.insert(callee.id, Res::Builtin(Builtin::Panjang));
                if args.len() != 1 {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_ARITY_MISMATCH,
                            format!("'panjang' butuh 1 argumen, diberi {}", args.len()),
                        )
                        .primary(call.span, "jumlah argumen salah"),
                    );
                    for a in args {
                        self.check_expr(a, None);
                    }
                    return Type::I64;
                }
                let at = self.check_expr(&args[0], None);
                if !at.is_error() && !matches!(at, Type::Array { .. } | Type::Str) {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_INVALID_OPERAND,
                            format!("'panjang' butuh larik atau teks, bukan '{}'", at.display()),
                        )
                        .primary(args[0].span, "bukan larik atau teks"),
                    );
                }
                Type::I64
            }
            Some(Res::Builtin(Builtin::Tampilkan)) => {
                self.results.name_res.insert(callee.id, Res::Builtin(Builtin::Tampilkan));
                if args.len() != 1 {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_ARITY_MISMATCH,
                            format!("'tampilkan' butuh 1 argumen, diberi {}", args.len()),
                        )
                        .primary(call.span, "jumlah argumen salah"),
                    );
                    for a in args {
                        self.check_expr(a, None);
                    }
                    return Type::Unit;
                }
                let at = self.check_expr(&args[0], None);
                if !at.is_error()
                    && !matches!(
                        at,
                        Type::Int { .. }
                            | Type::Float(_)
                            | Type::Bool
                            | Type::Char
                            | Type::Str
                            | Type::Enum(_)
                    )
                {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_INVALID_OPERAND,
                            format!("'tampilkan' tidak mendukung tipe '{}'", at.display()),
                        )
                        .primary(args[0].span, "tipe tidak bisa dicetak"),
                    );
                }
                Type::Unit
            }
            Some(Res::Func(idx)) => {
                self.results.name_res.insert(callee.id, Res::Func(idx));
                let sig = self.results.functions[idx].clone();
                let ok_arity = if sig.variadic {
                    args.len() >= sig.params.len()
                } else {
                    args.len() == sig.params.len()
                };
                if !ok_arity {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_ARITY_MISMATCH,
                            format!(
                                "fungsi '{}' butuh {} argumen, diberi {}",
                                sig.name,
                                sig.params.len(),
                                args.len()
                            ),
                        )
                        .primary(call.span, "jumlah argumen salah"),
                    );
                }
                for (i, a) in args.iter().enumerate() {
                    let expected = sig.params.get(i).map(|(_, t)| t.clone());
                    let at = self.check_expr(a, expected.as_ref());
                    if let Some(pt) = expected {
                        if !at.assignable_to(&pt) {
                            self.diags.emit(
                                Diagnostic::error(
                                    codes::E_TYPE_MISMATCH_ASSIGN,
                                    format!(
                                        "argumen ke-{} bertipe '{}', diharapkan '{}'",
                                        i + 1,
                                        at.display(),
                                        pt.display()
                                    ),
                                )
                                .primary(a.span, format!("bertipe '{}'", at.display())),
                            );
                        }
                    }
                }
                sig.ret.clone()
            }
            Some(Res::Local(_)) => self.check_indirect_call(call, callee, args),
            Some(_) | None => {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_NOT_CALLABLE,
                        format!("'{}' bukan fungsi yang bisa dipanggil", name),
                    )
                    .primary(callee.span, "bukan fungsi"),
                );
                for a in args {
                    self.check_expr(a, None);
                }
                Type::Error
            }
        }
    }

    fn check_indirect_call(&mut self, call: &Expr, callee: &Expr, args: &[Expr]) -> Type {
        let ct = self.check_expr(callee, None);
        let Type::Fn { params, ret } = ct.clone() else {
            if !ct.is_error() {
                self.diags.emit(
                    Diagnostic::error(codes::E_NOT_CALLABLE, "ekspresi ini tidak bisa dipanggil")
                        .primary(callee.span, "bukan sebuah fungsi"),
                );
            }
            for a in args {
                self.check_expr(a, None);
            }
            return Type::Error;
        };
        self.results.indirect_calls.insert(call.id);
        if args.len() != params.len() {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_ARITY_MISMATCH,
                    format!("nilai fungsi butuh {} argumen, diberi {}", params.len(), args.len()),
                )
                .primary(call.span, "jumlah argumen salah"),
            );
            for a in args {
                self.check_expr(a, None);
            }
            return *ret;
        }
        for (a, pt) in args.iter().zip(params.iter()) {
            let at = self.check_expr(a, Some(pt));
            if !at.is_error() && !at.assignable_to(pt) {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_TYPE_MISMATCH_ASSIGN,
                        format!("argumen bertipe '{}', diharapkan '{}'", at.display(), pt.display()),
                    )
                    .primary(a.span, format!("bertipe '{}'", at.display())),
                );
            }
        }
        *ret
    }

    fn check_generic_call(
        &mut self,
        gidx: usize,
        call: &Expr,
        callee: &Expr,
        args: &[Expr],
    ) -> Type {
        let template = self.generic_fns[gidx].clone();
        let type_params: Vec<String> =
            template.type_params.iter().map(|p| p.name.name.clone()).collect();

        if args.len() != template.params.len() {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_ARITY_MISMATCH,
                    format!(
                        "fungsi '{}' butuh {} argumen, diberi {}",
                        template.name.name,
                        template.params.len(),
                        args.len()
                    ),
                )
                .primary(call.span, "jumlah argumen salah"),
            );
            for a in args {
                self.check_expr(a, None);
            }
            return Type::Error;
        }

        let arg_types: Vec<Type> = args.iter().map(|a| self.check_expr(a, None)).collect();
        if arg_types.iter().any(|t| t.is_error()) {
            return Type::Error;
        }

        let mut subst: HashMap<String, Type> = HashMap::new();
        for (p, at) in template.params.iter().zip(arg_types.iter()) {
            let pat = self.resolve_type_generic(&p.ty, &type_params);
            if !unify(&pat, at, &mut subst) {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_TYPE_MISMATCH_ASSIGN,
                        format!(
                            "argumen bertipe '{}' tidak cocok dengan parameter '{}'",
                            at.display(),
                            pat.display()
                        ),
                    )
                    .primary(p.ty.span, "tipe parameter")
                    .secondary(call.span, "pada pemanggilan ini"),
                );
                return Type::Error;
            }
        }

        let type_args: Vec<Type> = match type_params
            .iter()
            .map(|name| subst.get(name).cloned())
            .collect::<Option<Vec<_>>>()
        {
            Some(v) => v,
            None => {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_CANNOT_INFER,
                        format!(
                            "tidak bisa menyimpulkan parameter tipe untuk '{}'",
                            template.name.name
                        ),
                    )
                    .primary(call.span, "parameter tipe tidak tersimpulkan dari argumen"),
                );
                return Type::Error;
            }
        };

        let mut bound_failed = false;
        for (tp, concrete) in template.type_params.iter().zip(type_args.iter()) {
            for bound in &tp.bounds {
                if !self.trait_ids.contains_key(&bound.name) {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_UNKNOWN_TRAIT_OR_TYPE,
                            format!("sifat '{}' tidak dikenal", bound.name),
                        )
                        .primary(bound.span, "bukan sifat yang dikenal"),
                    );
                    bound_failed = true;
                } else if !self.type_implements(concrete, &bound.name) {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_BOUND_NOT_SATISFIED,
                            format!(
                                "tipe '{}' tidak mengimplementasi sifat '{}'",
                                concrete.display(),
                                bound.name
                            ),
                        )
                        .primary(call.span, "batas sifat tidak terpenuhi")
                        .secondary(tp.name.span, "diminta di sini"),
                    );
                    bound_failed = true;
                }
            }
        }
        if bound_failed {
            return Type::Error;
        }

        let instance_idx = self.instantiate(gidx, &type_args);
        self.results.name_res.insert(callee.id, Res::Func(instance_idx));
        self.results.functions[instance_idx].ret.clone()
    }

    fn instantiate(&mut self, gidx: usize, type_args: &[Type]) -> usize {
        let key = (gidx, type_args.to_vec());
        if let Some(&idx) = self.instantiations.get(&key) {
            return idx;
        }
        let template = self.generic_fns[gidx].clone();
        let type_params: Vec<String> =
            template.type_params.iter().map(|p| p.name.name.clone()).collect();
        let subst: HashMap<String, Type> =
            type_params.iter().cloned().zip(type_args.iter().cloned()).collect();

        let mut inst = template.clone();
        inst.type_params.clear();
        inst.name.name = mangle_name(&template.name.name, type_args);
        substitute_fn_types(&mut inst, &subst, &self.struct_ids, &self.enum_ids, &self.results);
        self.renumber_fn(&mut inst);

        let params: Vec<(String, Type)> = inst
            .params
            .iter()
            .map(|p| (p.name.name.clone(), self.resolve_type(&p.ty)))
            .collect();
        let ret = inst.ret.as_ref().map(|t| self.resolve_type(t)).unwrap_or(Type::Unit);
        let sig = FnSig {
            name: inst.name.name.clone(),
            params,
            ret,
            is_extern: false,
            variadic: false,
            public: false,
            def_span: template.name.span,
        };

        let instance_idx = self.results.functions.len();
        self.results.functions.push(sig);
        self.globals.insert(inst.name.name.clone(), Res::Func(instance_idx));

        let mono_idx = self.results.mono_functions.len();
        self.results.mono_functions.push(inst);
        self.mono_worklist.push(mono_idx);
        self.instantiations.insert(key, instance_idx);
        instance_idx
    }

    fn resolve_type_generic(&self, ty: &TypeExpr, type_params: &[String]) -> Type {
        match &ty.kind {
            TypeExprKind::Named(name) => {
                if type_params.iter().any(|p| p == name) {
                    return Type::Param(name.clone());
                }
                if let Some(t) = Type::from_builtin_name(name) {
                    return t;
                }
                if let Some(&id) = self.struct_ids.get(name) {
                    return Type::Struct(id);
                }
                if let Some(&id) = self.enum_ids.get(name) {
                    return Type::Enum(id);
                }
                Type::Error
            }
            TypeExprKind::Array { elem, size } => Type::Array {
                elem: Box::new(self.resolve_type_generic(elem, type_params)),
                size: *size,
            },
            TypeExprKind::Applied { name, args } if name == "kotak" && args.len() == 1 => {
                Type::Kotak(Box::new(self.resolve_type_generic(&args[0], type_params)))
            }
            TypeExprKind::Applied { name, args } if name == "Hasil" && args.len() == 2 => {
                Type::Hasil {
                    ok: Box::new(self.resolve_type_generic(&args[0], type_params)),
                    err: Box::new(self.resolve_type_generic(&args[1], type_params)),
                }
            }
            TypeExprKind::Applied { .. } => Type::Error,
            TypeExprKind::Ref { inner, mutable } => Type::Ref {
                inner: Box::new(self.resolve_type_generic(inner, type_params)),
                mutable: *mutable,
            },
            TypeExprKind::Fn { params, ret } => Type::Fn {
                params: params.iter().map(|p| self.resolve_type_generic(p, type_params)).collect(),
                ret: Box::new(
                    ret.as_ref()
                        .map(|r| self.resolve_type_generic(r, type_params))
                        .unwrap_or(Type::Unit),
                ),
            },
        }
    }

    fn renumber_fn(&mut self, f: &mut Function) {
        renumber_block(&mut f.body, &mut self.next_mono_node);
        for p in &mut f.params {
            p.id = NodeId(self.next_mono_node);
            self.next_mono_node += 1;
        }
    }

    fn check_borrow(&mut self, call: &Expr, args: &[Expr], mutable: bool) -> Type {
        let name = if mutable { "ambil_ubah" } else { "ambil" };
        if args.len() != 1 {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_ARITY_MISMATCH,
                    format!("'{}' butuh 1 argumen, diberi {}", name, args.len()),
                )
                .primary(call.span, "jumlah argumen salah"),
            );
            for a in args {
                self.check_expr(a, None);
            }
            return Type::Error;
        }
        let target = &args[0];
        let ty = self.check_expr(target, None);
        if ty.is_error() {
            return Type::Error;
        }
        if !target.is_lvalue() {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_BORROW_NOT_PLACE,
                    format!("'{}' hanya bisa ambil reference dari sebuah variabel", name),
                )
                .primary(target.span, "ini bukan variabel yang bisa diambil reference-nya"),
            );
            return Type::Error;
        }
        if mutable && !self.place_is_mutable(target) {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_BORROW_IMMUTABLE,
                    "'&ubah' butuh variabel yang bisa diubah ('ubah')",
                )
                .primary(target.span, "ini dideklarasikan dengan 'nilai'")
                .help("pakai 'ubah' supaya bisa diambil reference mutable"),
            );
        }
        Type::Ref { inner: Box::new(ty), mutable }
    }

    fn require_hasil_copy(&mut self, ok: &Type, err: &Type, span: Span) -> bool {
        let bad = if !self.is_copy(ok) {
            Some(ok)
        } else if !self.is_copy(err) {
            Some(err)
        } else {
            None
        };
        if let Some(t) = bad {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_HASIL_NOT_COPY,
                    format!("'Hasil' belum mendukung tipe milik '{}' sebagai muatan", t.display()),
                )
                .primary(span, "muatan bertipe milik")
                .help("gunakan tipe salinan (mis. angka, teks) untuk nilai dan error"),
            );
            false
        } else {
            true
        }
    }

    fn check_hasil_builtin(
        &mut self,
        b: Builtin,
        call: &Expr,
        args: &[Expr],
        expected: Option<&Type>,
    ) -> Type {
        let want = if matches!(b, Builtin::NilaiAtau | Builtin::GagalAtau) { 2 } else { 1 };
        let name = match b {
            Builtin::Sukses => "sukses",
            Builtin::Gagal => "gagal",
            Builtin::Berhasil => "berhasil",
            Builtin::NilaiAtau => "nilai_atau",
            Builtin::GagalAtau => "gagal_atau",
            _ => unreachable!(),
        };
        if args.len() != want {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_ARITY_MISMATCH,
                    format!("'{}' butuh {} argumen, diberi {}", name, want, args.len()),
                )
                .primary(call.span, "jumlah argumen salah"),
            );
            for a in args {
                self.check_expr(a, None);
            }
            return Type::Error;
        }

        match b {
            Builtin::Sukses | Builtin::Gagal => {
                let exp = match expected {
                    Some(Type::Hasil { ok, err }) => Some(((**ok).clone(), (**err).clone())),
                    _ => None,
                };
                let is_ok = matches!(b, Builtin::Sukses);
                let hint = exp.as_ref().map(|(o, e)| if is_ok { o.clone() } else { e.clone() });
                let at = self.check_expr(&args[0], hint.as_ref());
                let Some((ok, err)) = exp else {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_HASIL_NO_CONTEXT,
                            format!("tipe 'Hasil' dari '{}' tidak bisa disimpulkan di sini", name),
                        )
                        .primary(call.span, "tidak ada tipe 'Hasil' yang diharapkan")
                        .help("beri anotasi tipe atau pakai di posisi kembalian 'Hasil<T, R>'"),
                    );
                    return Type::Error;
                };
                let target = if is_ok { &ok } else { &err };
                if !at.is_error() && !at.assignable_to(target) {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_HASIL_TYPE_CONFLICT,
                            format!("'{}' diberi '{}', tetapi 'Hasil' butuh '{}'", name, at.display(), target.display()),
                        )
                        .primary(args[0].span, format!("bertipe '{}'", at.display())),
                    );
                }
                self.require_hasil_copy(&ok, &err, call.span);
                Type::Hasil { ok: Box::new(ok), err: Box::new(err) }
            }
            Builtin::Berhasil => {
                let at = self.check_expr(&args[0], None);
                match at {
                    Type::Hasil { .. } => Type::Bool,
                    Type::Error => Type::Error,
                    other => {
                        self.diags.emit(
                            Diagnostic::error(
                                codes::E_INSPECT_MISUSE,
                                format!("'berhasil' butuh 'Hasil', bukan '{}'", other.display()),
                            )
                            .primary(args[0].span, "bukan 'Hasil'"),
                        );
                        Type::Error
                    }
                }
            }
            Builtin::NilaiAtau | Builtin::GagalAtau => {
                let at = self.check_expr(&args[0], None);
                let picked = match &at {
                    Type::Hasil { ok, err } => {
                        Some(if matches!(b, Builtin::NilaiAtau) { (**ok).clone() } else { (**err).clone() })
                    }
                    _ => None,
                };
                match picked {
                    Some(want_ty) => {
                        let fb = self.check_expr(&args[1], Some(&want_ty));
                        if !fb.is_error() && !fb.assignable_to(&want_ty) {
                            self.diags.emit(
                                Diagnostic::error(
                                    codes::E_INSPECT_MISUSE,
                                    format!("cadangan bertipe '{}', diharapkan '{}'", fb.display(), want_ty.display()),
                                )
                                .primary(args[1].span, "tipe cadangan tidak cocok"),
                            );
                        }
                        want_ty
                    }
                    None => {
                        self.check_expr(&args[1], None);
                        if !at.is_error() {
                            self.diags.emit(
                                Diagnostic::error(
                                    codes::E_INSPECT_MISUSE,
                                    format!("'{}' butuh 'Hasil', bukan '{}'", name, at.display()),
                                )
                                .primary(args[0].span, "bukan 'Hasil'"),
                            );
                        }
                        Type::Error
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    fn check_try(&mut self, inner: &Expr) -> Type {
        let it = self.check_expr(inner, None);
        let (ok, err) = match &it {
            Type::Hasil { ok, err } => ((**ok).clone(), (**err).clone()),
            Type::Error => return Type::Error,
            other => {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_TRY_NON_HASIL,
                        format!("'coba' hanya untuk 'Hasil', bukan '{}'", other.display()),
                    )
                    .primary(inner.span, "bukan 'Hasil'"),
                );
                return Type::Error;
            }
        };
        match self.current_ret.clone() {
            Type::Hasil { err: ret_err, .. } => {
                if *ret_err != err {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_TRY_BAD_CONTEXT,
                            format!(
                                "'coba' meneruskan error '{}', tetapi fungsi mengembalikan error '{}'",
                                err.display(),
                                ret_err.display()
                            ),
                        )
                        .primary(inner.span, "tipe error tidak cocok dengan kembalian"),
                    );
                }
            }
            _ => {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_TRY_BAD_CONTEXT,
                        "'coba' hanya boleh di dalam fungsi yang mengembalikan 'Hasil<T, R>'",
                    )
                    .primary(inner.span, "fungsi ini tidak mengembalikan 'Hasil'"),
                );
            }
        }
        ok
    }

    fn place_is_mutable(&self, expr: &Expr) -> bool {
        match &expr.kind {
            ExprKind::Name(_) => match self.results.name_res.get(&expr.id) {
                Some(Res::Local(id)) => self.results.locals[id].mutable,
                _ => false,
            },
            ExprKind::Field { base, .. } | ExprKind::Index { base, .. } => {
                match self.results.expr_type(base.id) {
                    Type::Ref { mutable, .. } => *mutable,
                    _ => self.place_is_mutable(base),
                }
            }
            ExprKind::Deref(inner) => {
                matches!(self.results.expr_type(inner.id), Type::Ref { mutable: true, .. })
            }
            _ => false,
        }
    }

    fn check_deref(&mut self, inner: &Expr) -> Type {
        let it = self.check_expr(inner, None);
        match self.ref_deref_target(&it) {
            Some((pointee, _)) => {
                if !self.is_copy(&pointee) {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_DEREF_NON_COPY,
                            format!(
                                "tidak bisa membaca nilai bertipe milik '{}' lewat reference",
                                pointee.display()
                            ),
                        )
                        .primary(inner.span, "isi reference bertipe milik"),
                    );
                }
                pointee
            }
            None => {
                if !it.is_error() {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_DEREF_NON_REF,
                            format!("'*' hanya untuk reference, bukan '{}'", it.display()),
                        )
                        .primary(inner.span, "bukan reference"),
                    );
                }
                Type::Error
            }
        }
    }

    fn check_enum_variant(&mut self, node: NodeId, eid: EnumId, variant: &Ident) -> Type {
        let info = self.results.enum_info(eid);
        match info.variant_tag(&variant.name) {
            Some(tag) => {
                self.results.enum_variant.insert(node, (eid, tag));
                Type::Enum(eid)
            }
            None => {
                let ename = info.name.clone();
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_UNKNOWN_VARIANT,
                        format!("enum '{}' tidak memiliki varian '{}'", ename, variant.name),
                    )
                    .primary(variant.span, "varian tidak dikenal"),
                );
                Type::Error
            }
        }
    }

    fn check_field(&mut self, base: &Expr, field: &Ident) -> Type {
        let bt = self.check_expr(base, None);
        let bt = match bt {
            Type::Ref { inner, .. } => (*inner).clone(),
            other => other,
        };
        match bt {
            Type::Struct(id) => {
                let info = self.results.struct_info(id);
                match info.field(&field.name) {
                    Some((_, f)) => f.ty.clone(),
                    None => {
                        let sname = info.name.clone();
                        self.diags.emit(
                            Diagnostic::error(
                                codes::E_UNKNOWN_FIELD,
                                format!("tipe '{}' tidak memiliki field '{}'", sname, field.name),
                            )
                            .primary(field.span, "field tidak dikenal"),
                        );
                        Type::Error
                    }
                }
            }
            Type::Error => Type::Error,
            other => {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_NOT_A_STRUCT,
                        format!("nilai bertipe '{}' bukan struct", other.display()),
                    )
                    .primary(base.span, "akses field hanya untuk struct"),
                );
                Type::Error
            }
        }
    }

    fn check_assignable(&mut self, target: &Expr) {
        match &target.kind {
            ExprKind::Name(_) => match self.results.name_res.get(&target.id) {
                Some(Res::Local(id)) => {
                    let info = &self.results.locals[id];
                    if !info.mutable {
                        let nm = info.name.clone();
                        self.diags.emit(
                            Diagnostic::error(
                                codes::E_ASSIGN_IMMUTABLE,
                                format!("'{}' immutable, tidak bisa diubah", nm),
                            )
                            .primary(target.span, "dideklarasikan dengan 'nilai'")
                            .help("pakai 'ubah' supaya bisa diubah"),
                        );
                    }
                }
                Some(_) => {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_INVALID_ASSIGN_TARGET,
                            "target penugasan tidak valid",
                        )
                        .primary(target.span, "bukan variabel"),
                    );
                }
                None => {}
            },
            ExprKind::Field { base, .. } | ExprKind::Index { base, .. } => {
                match self.results.expr_type(base.id).clone() {
                    Type::Ref { mutable: true, .. } => {}
                    Type::Ref { mutable: false, .. } => {
                        self.diags.emit(
                            Diagnostic::error(
                                codes::E_STORE_THROUGH_SHARED,
                                "tidak bisa menulis lewat reference '&' (harus '&ubah')",
                            )
                            .primary(target.span, "reference ini hanya untuk baca"),
                        );
                    }
                    _ => self.check_assignable(base),
                }
            }
            ExprKind::Deref(inner) => {
                let it = self.results.expr_type(inner.id).clone();
                if let Type::Ref { mutable, .. } = it {
                    if !mutable {
                        self.diags.emit(
                            Diagnostic::error(
                                codes::E_STORE_THROUGH_SHARED,
                                "tidak bisa menulis lewat reference '&' (harus '&ubah')",
                            )
                            .primary(target.span, "reference ini hanya untuk baca"),
                        );
                    }
                }
            }
            _ => {}
        }
    }

    fn resolve_type(&mut self, ty: &TypeExpr) -> Type {
        match &ty.kind {
            TypeExprKind::Named(name) => {
                if name == "Diri" {
                    return match &self.current_self {
                        Some(t) => t.clone(),
                        None => {
                            self.diags.emit(
                                Diagnostic::error(
                                    codes::E_UNKNOWN_TYPE,
                                    "'Diri' hanya bisa dipakai di dalam 'sifat' atau 'implementasi'",
                                )
                                .primary(ty.span, "di luar konteks sifat/implementasi"),
                            );
                            Type::Error
                        }
                    };
                }
                if let Some(t) = Type::from_builtin_name(name) {
                    return t;
                }
                if let Some(&id) = self.struct_ids.get(name) {
                    self.check_visibility(name, ty.span);
                    return Type::Struct(id);
                }
                if let Some(&id) = self.enum_ids.get(name) {
                    self.check_visibility(name, ty.span);
                    return Type::Enum(id);
                }
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_UNKNOWN_TYPE,
                        format!("tipe '{}' tidak dikenal", name),
                    )
                    .primary(ty.span, "tipe tidak ditemukan"),
                );
                Type::Error
            }
            TypeExprKind::Array { elem, size } => {
                let elem_ty = self.resolve_type(elem);
                Type::Array { elem: Box::new(elem_ty), size: *size }
            }
            TypeExprKind::Applied { name, args } => {
                if name == "kotak" && args.len() == 1 {
                    let inner = self.resolve_type(&args[0]);
                    return Type::Kotak(Box::new(inner));
                }
                if name == "Hasil" && args.len() == 2 {
                    let ok = self.resolve_type(&args[0]);
                    let err = self.resolve_type(&args[1]);
                    return Type::Hasil { ok: Box::new(ok), err: Box::new(err) };
                }
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_UNKNOWN_TYPE,
                        format!("tipe generik '{}' dengan {} argumen tidak dikenal", name, args.len()),
                    )
                    .primary(ty.span, "hanya 'kotak<T>' dan 'Hasil<T, R>' yang didukung saat ini"),
                );
                Type::Error
            }
            TypeExprKind::Ref { inner, mutable } => {
                let inner = self.resolve_type(inner);
                Type::Ref { inner: Box::new(inner), mutable: *mutable }
            }
            TypeExprKind::Fn { params, ret } => {
                let params = params.iter().map(|p| self.resolve_type(p)).collect();
                let ret = ret.as_ref().map(|r| self.resolve_type(r)).unwrap_or(Type::Unit);
                Type::Fn { params, ret: Box::new(ret) }
            }
        }
    }

    fn is_copy(&self, ty: &Type) -> bool {
        match ty {
            Type::Kotak(_) => false,
            Type::Struct(id) => {
                self.results.structs[id.0 as usize].fields.iter().all(|f| self.is_copy(&f.ty))
            }
            Type::Array { elem, .. } => self.is_copy(elem),
            Type::Hasil { ok, err } => self.is_copy(ok) && self.is_copy(err),
            Type::Param(_) => true,
            _ => true,
        }
    }

    fn ref_deref_target(&self, ty: &Type) -> Option<(Type, bool)> {
        if let Type::Ref { inner, mutable } = ty {
            Some(((**inner).clone(), *mutable))
        } else {
            None
        }
    }

    fn expect_bool(&mut self, t: &Type, span: Span, ctx: &str) {
        if !t.is_error() && *t != Type::Bool {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_INVALID_OPERAND,
                    format!("{} harus 'boolean', bukan '{}'", ctx, t.display()),
                )
                .primary(span, format!("bertipe '{}'", t.display())),
            );
        }
    }

    fn invalid_operand(&mut self, op: BinOp, span: Span, t: &Type) {
        self.diags.emit(
            Diagnostic::error(
                codes::E_INVALID_OPERAND,
                format!("operator '{}' tidak mendukung tipe '{}'", op.symbol(), t.display()),
            )
            .primary(span, "tipe operand tidak valid"),
        );
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare(&mut self, name: &str, res: Res, span: Span) {
        if let Some(scope) = self.scopes.last_mut() {
            if scope.contains_key(name) {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_REDEFINITION,
                        format!("'{}' sudah dideklarasikan pada scope ini", name),
                    )
                    .primary(span, "deklarasi ganda"),
                );
                return;
            }
            scope.insert(name.to_string(), res);
        }
    }

    fn lookup(&self, name: &str) -> Option<Res> {
        for scope in self.scopes.iter().rev() {
            if let Some(res) = scope.get(name) {
                return Some(res.clone());
            }
        }
        self.globals.get(name).cloned()
    }

    fn alloc_local(&mut self, name: String, ty: Type, mutable: bool) -> LocalId {
        let id = LocalId(self.next_local);
        self.next_local += 1;
        self.results.locals.insert(id, LocalInfo { name, ty, mutable });
        id
    }
}

fn is_builtin_name(name: &str) -> bool {
    matches!(
        name,
        "tampilkan"
            | "panjang"
            | "kotak"
            | "isi"
            | "ambil"
            | "ambil_ubah"
            | "sukses"
            | "gagal"
            | "berhasil"
            | "nilai_atau"
            | "gagal_atau"
    )
}

fn unify(pat: &Type, actual: &Type, subst: &mut HashMap<String, Type>) -> bool {
    match pat {
        Type::Param(name) => match subst.get(name) {
            Some(bound) => bound == actual,
            None => {
                subst.insert(name.clone(), actual.clone());
                true
            }
        },
        Type::Array { elem, size } => {
            if let Type::Array { elem: e2, size: s2 } = actual {
                size == s2 && unify(elem, e2, subst)
            } else {
                false
            }
        }
        Type::Ref { inner, mutable } => {
            if let Type::Ref { inner: i2, mutable: m2 } = actual {
                mutable == m2 && unify(inner, i2, subst)
            } else {
                false
            }
        }
        Type::Hasil { ok, err } => {
            if let Type::Hasil { ok: o2, err: e2 } = actual {
                unify(ok, o2, subst) && unify(err, e2, subst)
            } else {
                false
            }
        }
        other => other == actual,
    }
}

fn mangle_name(base: &str, type_args: &[Type]) -> String {
    let parts: Vec<String> = type_args
        .iter()
        .map(|t| t.display().chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect())
        .collect();
    format!("{}${}", base, parts.join("_"))
}

fn sanitize(s: &str) -> String {
    s.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect()
}

fn mangle_method(self_type: &Type, trait_name: Option<&str>, method: &str) -> String {
    format!("{}${}${}", sanitize(&self_type.display()), trait_name.unwrap_or(""), method)
}

fn subst_type(ty: &Type, subst: &HashMap<String, Type>) -> Type {
    match ty {
        Type::Param(n) => subst.get(n).cloned().unwrap_or_else(|| ty.clone()),
        Type::Array { elem, size } => {
            Type::Array { elem: Box::new(subst_type(elem, subst)), size: *size }
        }
        Type::Kotak(inner) => Type::Kotak(Box::new(subst_type(inner, subst))),
        Type::Ref { inner, mutable } => {
            Type::Ref { inner: Box::new(subst_type(inner, subst)), mutable: *mutable }
        }
        Type::Hasil { ok, err } => Type::Hasil {
            ok: Box::new(subst_type(ok, subst)),
            err: Box::new(subst_type(err, subst)),
        },
        Type::Fn { params, ret } => Type::Fn {
            params: params.iter().map(|p| subst_type(p, subst)).collect(),
            ret: Box::new(subst_type(ret, subst)),
        },
        other => other.clone(),
    }
}

fn dummy_span() -> Span {
    Span::point(lime_span::FileId(0), 0)
}

fn type_to_type_expr_kind(ty: &Type, results: &TypeckResults) -> TypeExprKind {
    match ty {
        Type::Struct(id) => TypeExprKind::Named(results.structs[id.0 as usize].name.clone()),
        Type::Enum(id) => TypeExprKind::Named(results.enums[id.0 as usize].name.clone()),
        Type::Array { elem, size } => TypeExprKind::Array {
            elem: Box::new(TypeExpr {
                kind: type_to_type_expr_kind(elem, results),
                span: dummy_span(),
            }),
            size: *size,
        },
        Type::Kotak(inner) => TypeExprKind::Applied {
            name: "kotak".to_string(),
            args: vec![TypeExpr {
                kind: type_to_type_expr_kind(inner, results),
                span: dummy_span(),
            }],
        },
        Type::Ref { inner, mutable } => TypeExprKind::Ref {
            inner: Box::new(TypeExpr {
                kind: type_to_type_expr_kind(inner, results),
                span: dummy_span(),
            }),
            mutable: *mutable,
        },
        Type::Hasil { ok, err } => TypeExprKind::Applied {
            name: "Hasil".to_string(),
            args: vec![
                TypeExpr { kind: type_to_type_expr_kind(ok, results), span: dummy_span() },
                TypeExpr { kind: type_to_type_expr_kind(err, results), span: dummy_span() },
            ],
        },
        other => TypeExprKind::Named(other.display()),
    }
}

fn substitute_type_expr(te: &mut TypeExpr, subst: &HashMap<String, Type>, results: &TypeckResults) {
    let replacement = if let TypeExprKind::Named(name) = &te.kind {
        subst.get(name).map(|c| type_to_type_expr_kind(c, results))
    } else {
        None
    };
    if let Some(kind) = replacement {
        te.kind = kind;
        return;
    }
    match &mut te.kind {
        TypeExprKind::Array { elem, .. } => substitute_type_expr(elem, subst, results),
        TypeExprKind::Applied { args, .. } => {
            for a in args {
                substitute_type_expr(a, subst, results);
            }
        }
        TypeExprKind::Ref { inner, .. } => substitute_type_expr(inner, subst, results),
        TypeExprKind::Fn { params, ret } => {
            for p in params {
                substitute_type_expr(p, subst, results);
            }
            if let Some(r) = ret {
                substitute_type_expr(r, subst, results);
            }
        }
        TypeExprKind::Named(_) => {}
    }
}

fn substitute_fn_types(
    f: &mut Function,
    subst: &HashMap<String, Type>,
    _struct_ids: &HashMap<String, StructId>,
    _enum_ids: &HashMap<String, EnumId>,
    results: &TypeckResults,
) {
    for p in &mut f.params {
        substitute_type_expr(&mut p.ty, subst, results);
    }
    if let Some(ret) = &mut f.ret {
        substitute_type_expr(ret, subst, results);
    }
    substitute_block_lets(&mut f.body, subst, results);
}

fn substitute_block_lets(
    block: &mut Block,
    subst: &HashMap<String, Type>,
    results: &TypeckResults,
) {
    for stmt in &mut block.stmts {
        match stmt {
            Stmt::Let { ty: Some(t), .. } => substitute_type_expr(t, subst, results),
            Stmt::If(i) => {
                substitute_block_lets(&mut i.then_block, subst, results);
                for (_, b) in &mut i.else_ifs {
                    substitute_block_lets(b, subst, results);
                }
                if let Some(b) = &mut i.else_block {
                    substitute_block_lets(b, subst, results);
                }
            }
            Stmt::While { body, .. } => substitute_block_lets(body, subst, results),
            Stmt::For(f) => substitute_block_lets(&mut f.body, subst, results),
            Stmt::ForEach(f) => substitute_block_lets(&mut f.body, subst, results),
            Stmt::Block(b) => substitute_block_lets(b, subst, results),
            _ => {}
        }
    }
}

fn renumber_block(block: &mut Block, counter: &mut u32) {
    for stmt in &mut block.stmts {
        renumber_stmt(stmt, counter);
    }
}

fn renumber_stmt(stmt: &mut Stmt, counter: &mut u32) {
    match stmt {
        Stmt::Let { init, id, .. } => {
            renumber_expr(init, counter);
            *id = NodeId(*counter);
            *counter += 1;
        }
        Stmt::Assign { target, value, .. } => {
            renumber_expr(target, counter);
            renumber_expr(value, counter);
        }
        Stmt::Expr(e) => renumber_expr(e, counter),
        Stmt::If(i) => {
            renumber_expr(&mut i.cond, counter);
            renumber_block(&mut i.then_block, counter);
            for (c, b) in &mut i.else_ifs {
                renumber_expr(c, counter);
                renumber_block(b, counter);
            }
            if let Some(b) = &mut i.else_block {
                renumber_block(b, counter);
            }
        }
        Stmt::While { cond, body, .. } => {
            renumber_expr(cond, counter);
            renumber_block(body, counter);
        }
        Stmt::For(f) => {
            renumber_expr(&mut f.start, counter);
            renumber_expr(&mut f.end, counter);
            f.var_id = NodeId(*counter);
            *counter += 1;
            renumber_block(&mut f.body, counter);
        }
        Stmt::ForEach(f) => {
            renumber_expr(&mut f.iterable, counter);
            f.var_id = NodeId(*counter);
            *counter += 1;
            renumber_block(&mut f.body, counter);
        }
        Stmt::Return { value: Some(e), .. } => renumber_expr(e, counter),
        Stmt::Return { .. } | Stmt::Break(_) | Stmt::Continue(_) => {}
        Stmt::Block(b) => renumber_block(b, counter),
    }
}

fn renumber_expr(e: &mut Expr, counter: &mut u32) {
    e.id = NodeId(*counter);
    *counter += 1;
    match &mut e.kind {
        ExprKind::Binary { lhs, rhs, .. } => {
            renumber_expr(lhs, counter);
            renumber_expr(rhs, counter);
        }
        ExprKind::Unary { operand, .. } => renumber_expr(operand, counter),
        ExprKind::Call { callee, args } => {
            renumber_expr(callee, counter);
            for a in args {
                renumber_expr(a, counter);
            }
        }
        ExprKind::Field { base, .. } => renumber_expr(base, counter),
        ExprKind::StructLit { fields, .. } => {
            for f in fields {
                renumber_expr(&mut f.value, counter);
            }
        }
        ExprKind::ArrayLit(elems) => {
            for a in elems {
                renumber_expr(a, counter);
            }
        }
        ExprKind::Index { base, index } => {
            renumber_expr(base, counter);
            renumber_expr(index, counter);
        }
        ExprKind::Deref(inner) => renumber_expr(inner, counter),
        ExprKind::Try(inner) => renumber_expr(inner, counter),
        ExprKind::Int { .. }
        | ExprKind::Float { .. }
        | ExprKind::Str(_)
        | ExprKind::Char(_)
        | ExprKind::Bool(_)
        | ExprKind::Name(_) => {}
    }
}

fn block_always_returns(block: &Block) -> bool {
    block.stmts.iter().any(stmt_diverges)
}

fn stmt_diverges(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return { .. } => true,
        Stmt::Block(b) => block_always_returns(b),
        Stmt::If(i) => if_diverges(i),
        _ => false,
    }
}

fn if_diverges(i: &IfStmt) -> bool {
    i.else_block.is_some()
        && block_always_returns(&i.then_block)
        && i.else_ifs.iter().all(|(_, b)| block_always_returns(b))
        && i.else_block.as_ref().map(block_always_returns).unwrap_or(false)
}

fn stmt_is_terminator(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return { .. } | Stmt::Break(_) | Stmt::Continue(_) => true,
        Stmt::If(i) => if_diverges(i),
        _ => false,
    }
}

fn stmt_span(stmt: &Stmt) -> Span {
    match stmt {
        Stmt::Let { span, .. } => *span,
        Stmt::Assign { span, .. } => *span,
        Stmt::Expr(e) => e.span,
        Stmt::If(i) => i.span,
        Stmt::While { span, .. } => *span,
        Stmt::For(f) => f.span,
        Stmt::ForEach(f) => f.span,
        Stmt::Return { span, .. } => *span,
        Stmt::Break(s) => *s,
        Stmt::Continue(s) => *s,
        Stmt::Block(b) => b.span,
    }
}
