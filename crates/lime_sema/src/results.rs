use lime_ast::NodeId;
use lime_span::Span;
use lime_types::{StructId, Type};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct LocalId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Builtin {
    Tampilkan,
}

#[derive(Clone, Debug)]
pub enum Res {
    Local(LocalId),
    Func(usize),
    Builtin(Builtin),
    Struct(StructId),
}

#[derive(Clone, Debug)]
pub struct FnSig {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub ret: Type,
    pub is_extern: bool,
    pub variadic: bool,
    pub public: bool,
    pub def_span: Span,
}

impl FnSig {
    pub fn fn_type(&self) -> Type {
        Type::Fn {
            params: self.params.iter().map(|(_, t)| t.clone()).collect(),
            ret: Box::new(self.ret.clone()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FieldInfo {
    pub name: String,
    pub ty: Type,
    pub public: bool,
}

#[derive(Clone, Debug)]
pub struct StructInfo {
    pub name: String,
    pub fields: Vec<FieldInfo>,
    pub def_span: Span,
}

impl StructInfo {
    pub fn field(&self, name: &str) -> Option<(usize, &FieldInfo)> {
        self.fields.iter().enumerate().find(|(_, f)| f.name == name)
    }
}

#[derive(Clone, Debug)]
pub struct LocalInfo {
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
}

#[derive(Default)]
pub struct TypeckResults {
    pub functions: Vec<FnSig>,
    pub structs: Vec<StructInfo>,
    pub expr_types: HashMap<NodeId, Type>,
    pub name_res: HashMap<NodeId, Res>,
    pub local_of_binding: HashMap<Span, LocalId>,
    pub locals: HashMap<LocalId, LocalInfo>,
    pub main_present: bool,
}

impl TypeckResults {
    pub fn expr_type(&self, id: NodeId) -> &Type {
        self.expr_types.get(&id).unwrap_or(&Type::Error)
    }

    pub fn struct_info(&self, id: StructId) -> &StructInfo {
        &self.structs[id.0 as usize]
    }

    pub fn func_by_name(&self, name: &str) -> Option<(usize, &FnSig)> {
        self.functions.iter().enumerate().find(|(_, f)| f.name == name)
    }
}
