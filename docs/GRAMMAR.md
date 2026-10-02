# Exotickik Grammar (EBNF)

This is the concrete grammar the bootstrap parser implements. Notation:
`{ x }` = zero or more, `[ x ]` = optional, `|` = alternation, `'x'` = terminal.
Whitespace and comments are insignificant between tokens. Exotickik is
newline-insensitive except that statements are separated by their own structure
(no semicolons required); an optional `;` is accepted as a statement terminator.

```ebnf
program        = ( "program" | "modul" ) Ident { item } ;   (* "modul" for a library file *)

item           = function
               | struct
               | enum
               | trait
               | impl_block
               | main_block
               | extern_decl
               | use_decl
               | import_decl ;

trait          = [ "publik" ] "sifat" Ident "{" { method_sig } "}" ;
method_sig     = "fungsi" Ident "(" [ param_list ] ")" [ "->" type ] ;   (* no body *)

impl_block     = "implementasi" [ Ident "untuk" ] Ident "{" { function } "}" ;

use_decl       = "gunakan" Ident ;
import_decl    = "impor" Ident ;
extern_decl    = "ekstern" "fungsi" Ident "(" [ param_list [ "," "..." ] ] ")"
                 [ "->" type ] ;

main_block     = "utama" block ;

function       = [ "publik" ] "fungsi" Ident [ type_params ]
                 "(" [ param_list ] ")" [ "->" type ] block ;
type_params    = "<" type_param { "," type_param } ">" ;
type_param     = Ident [ ":" Ident { "+" Ident } ] ;      (* optional trait bounds *)
param_list     = param { "," param } ;
param          = Ident ":" type ;

struct         = [ "publik" ] "tipe" Ident [ type_params ] "{" { field } "}" ;
field          = [ "publik" ] Ident ":" type ;

type           = Ident [ "<" type { "," type } ">" ]   (* named or applied, e.g. kotak<i64> *)
               | "[" type ";" IntLit "]"                (* fixed array [T; N] *)
               | "&" [ "ubah" ] type                    (* reference: &T shared, &ubah T mutable *)
               | "fungsi" "(" [ type { "," type } ] ")" [ "->" type ] ;  (* function value type *)

block          = "{" { statement } "}" ;

statement      = let_stmt
               | assign_or_expr_stmt
               | if_stmt
               | while_stmt
               | for_stmt
               | match_stmt
               | return_stmt
               | "berhenti"
               | "lanjut"
               | block ;

let_stmt       = ( "nilai" | "ubah" ) Ident [ ":" type ] "=" expr ;
return_stmt    = "kembalikan" [ expr ] ;

if_stmt        = "jika" expr block
                 { "lain" "jika" expr block }
                 [ "lain" block ] ;

while_stmt     = "selama" expr block ;

for_stmt       = "untuk" Ident "dari" expr ( "sampai" | "hingga" ) expr block ;

match_stmt     = "cocok" expr "{"
                 { Ident "=>" block [ "," ] }
                 [ "lain" "=>" block [ "," ] ]
                 "}" ;                           (* arms: enum variants; must be exhaustive *)

assign_or_expr_stmt
               = expr [ "=" expr ] ;            (* lhs must be assignable *)

(* Expressions, lowest to highest precedence *)
expr           = or_expr ;
or_expr        = and_expr { ( "atau" | "||" ) and_expr } ;
and_expr       = eq_expr  { ( "dan"  | "&&" ) eq_expr } ;
eq_expr        = rel_expr { ( "==" | "!=" ) rel_expr } ;
rel_expr       = add_expr { ( "<" | "<=" | ">" | ">=" ) add_expr } ;
add_expr       = mul_expr { ( "+" | "-" ) mul_expr } ;
mul_expr       = cast     { ( "*" | "/" | "%" ) cast } ;
cast           = unary { "sebagai" type } ;            (* numeric/char conversion *)
unary          = ( "-" | "!" | "bukan" | "*" | "coba" ) unary
                                                       (* "*" derefs a reference; "coba" propagates a Hasil *)
               | postfix ;
postfix        = primary { call_suffix | field_suffix } ;
call_suffix    = "(" [ arg_list ] ")" ;
field_suffix   = "." Ident ;
arg_list       = expr { "," expr } ;
primary        = literal
               | Ident
               | "(" expr ")" ;

literal        = IntLit | FloatLit | StringLit | CharLit | "benar" | "salah" ;
```

## Notes on disambiguation
- `jika ... { }` — the parser requires a block, so there is no dangling-else
  ambiguity; `lain jika` chains are parsed iteratively.
- `assign_or_expr_stmt` first parses a full expression; if a `=` follows, the
  parsed expression must be an *l-value* (a bare identifier, a field or index
  access, or a dereference `*r`), otherwise `E0301 (target penugasan tidak
  valid)` is raised.
- References are created with the builtins `ambil(x)` (shared `&T`) and
  `ambil_ubah(x)` (mutable `&ubah T`); these are ordinary call syntax, resolved
  by the checker rather than the grammar. `*r` reads through a reference and,
  as an l-value, writes through a `&ubah` reference.
- A **method call** `x.m(args)` needs no dedicated grammar: it parses as a call
  whose callee is a field access (`postfix` chaining `field_suffix` then
  `call_suffix`). The checker treats a call whose callee is `base.name` as a
  method call, resolving `name` against the implementations for the type of
  `base` and passing `base` as the receiver.
- `Diri` is a contextual type name (not a keyword) usable only inside a `sifat`
  or `implementasi`, standing for the implementing type; `&Diri` / `&ubah Diri`
  / `Diri` choose the receiver's borrow mode.
- `Hasil<T, R>` is the built-in result type (an `Applied` type). `sukses`,
  `gagal`, `berhasil`, `nilai_atau`, and `gagal_atau` are reserved builtins used
  with ordinary call syntax; `coba e` is the propagation operator.
- `daftar`, `dorong`, `elemen`, `atur`, `bita_pada`, `cetak_bita`, `baca_file`,
  and `tulis_file` are reserved builtins called with ordinary call syntax;
  `baca_file`/`tulis_file` do whole-file I/O over `daftar<bita>` buffers.
- Statement boundaries are determined structurally; a leading `-` on a new line
  is parsed as a binary operator continuing the previous expression only inside
  an unclosed `(` — top-level statements do not span lines implicitly.
