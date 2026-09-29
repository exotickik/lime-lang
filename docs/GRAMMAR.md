# Lime Grammar (EBNF)

This is the concrete grammar the bootstrap parser implements. Notation:
`{ x }` = zero or more, `[ x ]` = optional, `|` = alternation, `'x'` = terminal.
Whitespace and comments are insignificant between tokens. Lime is
newline-insensitive except that statements are separated by their own structure
(no semicolons required); an optional `;` is accepted as a statement terminator.

```ebnf
program        = "program" Ident { item } ;

item           = function
               | struct
               | main_block
               | extern_decl
               | use_decl
               | import_decl ;

use_decl       = "gunakan" Ident ;
import_decl    = "impor" Ident ;
extern_decl    = "ekstern" "fungsi" Ident "(" [ param_list [ "," "..." ] ] ")"
                 [ "->" type ] ;

main_block     = "utama" block ;

function       = [ "publik" ] "fungsi" Ident [ "<" Ident { "," Ident } ">" ]
                 "(" [ param_list ] ")" [ "->" type ] block ;
param_list     = param { "," param } ;
param          = Ident ":" type ;

struct         = [ "publik" ] "tipe" Ident "{" { field } "}" ;
field          = [ "publik" ] Ident ":" type ;

type           = Ident [ "<" type { "," type } ">" ]   (* named or applied, e.g. kotak<i64> *)
               | "[" type ";" IntLit "]" ;              (* fixed array [T; N] *)

block          = "{" { statement } "}" ;

statement      = let_stmt
               | assign_or_expr_stmt
               | if_stmt
               | while_stmt
               | for_stmt
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

assign_or_expr_stmt
               = expr [ "=" expr ] ;            (* lhs must be assignable *)

(* Expressions, lowest to highest precedence *)
expr           = or_expr ;
or_expr        = and_expr { ( "atau" | "||" ) and_expr } ;
and_expr       = eq_expr  { ( "dan"  | "&&" ) eq_expr } ;
eq_expr        = rel_expr { ( "==" | "!=" ) rel_expr } ;
rel_expr       = add_expr { ( "<" | "<=" | ">" | ">=" ) add_expr } ;
add_expr       = mul_expr { ( "+" | "-" ) mul_expr } ;
mul_expr       = unary    { ( "*" | "/" | "%" ) unary } ;
unary          = ( "-" | "!" | "bukan" ) unary
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
  parsed expression must be an *l-value* (a bare identifier or a field access),
  otherwise `E0301 (target penugasan tidak valid)` is raised.
- Statement boundaries are determined structurally; a leading `-` on a new line
  is parsed as a binary operator continuing the previous expression only inside
  an unclosed `(` — top-level statements do not span lines implicitly.
