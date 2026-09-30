use lime_ast::NodeId;
use lime_span::Span;
use lime_types::{EnumId, StructId, Type};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct LocalId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Builtin {
    Tampilkan,
    Panjang,
    Kotak,
    Isi,
    Ambil,
    AmbilUbah,
    Sukses,
    Gagal,
    Berhasil,
    NilaiAtau,
    GagalAtau,
}

#[derive(Clone, Debug)]
pub enum Res {
    Local(LocalId),
    Func(usize),
    GenericFn(usize),
    Builtin(Builtin),
    Struct(StructId),
    Enum(EnumId),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RecvMode {
    Value,
    Ref,
    RefMut,
}

#[derive(Clone, Debug)]
pub struct MethodResolution {
    pub func_idx: usize,
    pub recv_mode: RecvMode,
}

#[derive(Clone, Debug)]
pub struct TraitMethodInfo {
    pub name: String,
    pub params: Vec<Type>,
    pub ret: Type,
}

#[derive(Clone, Debug)]
pub struct TraitInfo {
    pub name: String,
    pub methods: Vec<TraitMethodInfo>,
    pub def_span: Span,
}

impl TraitInfo {
    pub fn method(&self, name: &str) -> Option<&TraitMethodInfo> {
        self.methods.iter().find(|m| m.name == name)
    }
}

#[derive(Clone, Debug)]
pub struct ImplInfo {
    pub trait_name: Option<String>,
    pub self_type: Type,
    pub methods: HashMap<String, usize>,
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
pub struct EnumInfo {
    pub name: String,
    pub variants: Vec<String>,
    pub def_span: Span,
}

impl EnumInfo {
    pub fn variant_tag(&self, name: &str) -> Option<u32> {
        self.variants.iter().position(|v| v == name).map(|i| i as u32)
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
    pub enums: Vec<EnumInfo>,
    pub traits: Vec<TraitInfo>,
    pub impls: Vec<ImplInfo>,
    pub method_calls: HashMap<NodeId, MethodResolution>,
    pub indirect_calls: std::collections::HashSet<NodeId>,
    pub func_values: std::collections::HashSet<NodeId>,
    pub expr_types: HashMap<NodeId, Type>,
    pub name_res: HashMap<NodeId, Res>,
    pub enum_variant: HashMap<NodeId, (EnumId, u32)>,
    pub local_of_binding: HashMap<NodeId, LocalId>,
    pub locals: HashMap<LocalId, LocalInfo>,
    pub mono_functions: Vec<lime_ast::Function>,
    pub moved_uses: std::collections::HashSet<NodeId>,
    pub main_present: bool,
}

impl TypeckResults {
    pub fn expr_type(&self, id: NodeId) -> &Type {
        self.expr_types.get(&id).unwrap_or(&Type::Error)
    }

    pub fn struct_info(&self, id: StructId) -> &StructInfo {
        &self.structs[id.0 as usize]
    }

    pub fn enum_info(&self, id: EnumId) -> &EnumInfo {
        &self.enums[id.0 as usize]
    }

    pub fn func_by_name(&self, name: &str) -> Option<(usize, &FnSig)> {
        self.functions.iter().enumerate().find(|(_, f)| f.name == name)
    }
}
