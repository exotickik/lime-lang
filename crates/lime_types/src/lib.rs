#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct StructId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct EnumId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum IntWidth {
    W8,
    W16,
    W32,
    W64,
}

impl IntWidth {
    pub fn bits(self) -> u32 {
        match self {
            IntWidth::W8 => 8,
            IntWidth::W16 => 16,
            IntWidth::W32 => 32,
            IntWidth::W64 => 64,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum FloatWidth {
    W32,
    W64,
}

impl FloatWidth {
    pub fn bits(self) -> u32 {
        match self {
            FloatWidth::W32 => 32,
            FloatWidth::W64 => 64,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Type {
    Int { width: IntWidth, signed: bool },
    Float(FloatWidth),
    Bool,
    Char,
    Str,
    Unit,
    Struct(StructId),
    Enum(EnumId),
    Array { elem: Box<Type>, size: u64 },
    Fn { params: Vec<Type>, ret: Box<Type> },

    Error,
}

impl Type {
    pub const I8: Type = Type::Int { width: IntWidth::W8, signed: true };
    pub const I16: Type = Type::Int { width: IntWidth::W16, signed: true };
    pub const I32: Type = Type::Int { width: IntWidth::W32, signed: true };
    pub const I64: Type = Type::Int { width: IntWidth::W64, signed: true };
    pub const U8: Type = Type::Int { width: IntWidth::W8, signed: false };
    pub const U16: Type = Type::Int { width: IntWidth::W16, signed: false };
    pub const U32: Type = Type::Int { width: IntWidth::W32, signed: false };
    pub const U64: Type = Type::Int { width: IntWidth::W64, signed: false };
    pub const F32: Type = Type::Float(FloatWidth::W32);
    pub const F64: Type = Type::Float(FloatWidth::W64);

    pub fn is_numeric(&self) -> bool {
        matches!(self, Type::Int { .. } | Type::Float(_))
    }

    pub fn is_integer(&self) -> bool {
        matches!(self, Type::Int { .. })
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Type::Float(_))
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Type::Error)
    }

    pub fn is_signed_int(&self) -> bool {
        matches!(self, Type::Int { signed: true, .. })
    }

    pub fn assignable_to(&self, target: &Type) -> bool {
        if self.is_error() || target.is_error() {
            return true;
        }
        self == target
    }

    pub fn from_builtin_name(name: &str) -> Option<Type> {
        Some(match name {
            "i8" => Type::I8,
            "i16" => Type::I16,
            "i32" => Type::I32,
            "i64" | "angka" => Type::I64,
            "u8" | "bita" => Type::U8,
            "u16" => Type::U16,
            "u32" => Type::U32,
            "u64" => Type::U64,
            "f32" => Type::F32,
            "f64" => Type::F64,
            "boolean" | "bool" => Type::Bool,
            "karakter" | "char" => Type::Char,
            "teks" | "string" => Type::Str,
            "hampa" => Type::Unit,
            _ => return None,
        })
    }

    pub fn display(&self) -> String {
        match self {
            Type::Int { width, signed } => {
                format!("{}{}", if *signed { "i" } else { "u" }, width.bits())
            }
            Type::Float(w) => format!("f{}", w.bits()),
            Type::Bool => "boolean".into(),
            Type::Char => "karakter".into(),
            Type::Str => "teks".into(),
            Type::Unit => "hampa".into(),
            Type::Struct(id) => format!("tipe#{}", id.0),
            Type::Enum(id) => format!("enum#{}", id.0),
            Type::Array { elem, size } => format!("[{}; {}]", elem.display(), size),
            Type::Fn { params, ret } => {
                let ps: Vec<String> = params.iter().map(|t| t.display()).collect();
                format!("fungsi({}) -> {}", ps.join(", "), ret.display())
            }
            Type::Error => "<error>".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_names_and_aliases() {
        assert_eq!(Type::from_builtin_name("angka"), Some(Type::I64));
        assert_eq!(Type::from_builtin_name("i32"), Some(Type::I32));
        assert_eq!(Type::from_builtin_name("u8"), Some(Type::U8));
        assert_eq!(Type::from_builtin_name("bita"), Some(Type::U8));
        assert_eq!(Type::from_builtin_name("teks"), Some(Type::Str));
        assert_eq!(Type::from_builtin_name("boolean"), Some(Type::Bool));
        assert_eq!(Type::from_builtin_name("tidakada"), None);
    }

    #[test]
    fn assignability_is_invariant_but_error_absorbs() {
        assert!(Type::I32.assignable_to(&Type::I32));
        assert!(!Type::I32.assignable_to(&Type::I64));
        assert!(Type::Error.assignable_to(&Type::I64));
        assert!(Type::I64.assignable_to(&Type::Error));
    }

    #[test]
    fn display_names() {
        assert_eq!(Type::I64.display(), "i64");
        assert_eq!(Type::U8.display(), "u8");
        assert_eq!(Type::F32.display(), "f32");
        assert_eq!(Type::Str.display(), "teks");
    }
}
