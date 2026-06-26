# Midori Grammar Reference

This document defines the syntax of the Midori programming language.

## Notation

- `|` separates alternatives
- `[]` denotes optional
- `{}` denotes repetition (zero or more)
- `UPPERCASE` denotes tokens from the lexer

## Program Structure

```
program     = { newline | stmt }
stmt        = fn_def | let_binding | type_def | impl_block
            | trait_def | if_stmt | while_stmt | for_stmt
            | loop_stmt | break_stmt | continue_stmt
            | return_stmt | import_stmt | expr_stmt
```

## Functions

```
fn_def      = 'fn' IDENT '(' [fn_params] ')' ['->' type_expr] block
fn_params   = fn_param { ',' fn_param }
fn_param    = IDENT [':' type_expr] ['=' expr]
expr_fn     = 'fn' '(' [fn_params] ')' ['->' type_expr] expr

block       = '{' { newline | stmt } '}'
```

## Statements

```
let_binding = 'let' ['mut'] IDENT [':' type_expr] '=' expr
return_stmt = 'return' [expr]
if_stmt     = 'if' expr block ['else' (if_stmt | block)]
while_stmt  = 'while' expr block
for_stmt    = 'for' IDENT 'in' expr block
loop_stmt   = 'loop' block
break_stmt  = 'break' [expr]
continue_stmt = 'continue'
import_stmt = 'import' IDENT { '.' IDENT } ['as' IDENT]
```

## Expressions (in order of precedence)

```
expr        = pipe_expr
pipe_expr   = assign_expr { '|>' assign_expr }
assign_expr = or_expr { ('=' | '+=' | '-=' | '*=' | '/=') or_expr }
or_expr     = and_expr { ('or' | '||') and_expr }
and_expr    = comp_expr { ('and' | '&&') comp_expr }
comp_expr   = add_expr { ('==' | '!=' | '<' | '>' | '<=' | '>=') add_expr }
add_expr    = mul_expr { ('+' | '-') mul_expr }
mul_expr    = unary_expr { ('*' | '/' | '%') unary_expr }
unary_expr  = ('-' | 'not' | '!') unary_expr | call_expr
call_expr   = primary { '(' [expr_list] ')' | '.' IDENT ['(' [expr_list] ')'] | '[' expr ']' }
primary     = literal | IDENT | block | if_expr | match_expr
            | '(' expr ')' | '[' [expr_list] ']' | expr_fn | import_path

literal     = INT | FLOAT | STR | CHAR | 'true' | 'false' | 'nil'
if_expr     = 'if' expr block ['else' (if_expr | block)]
match_expr  = 'match' expr '{' { match_arm } '}'
match_arm   = pattern '=>' expr [',']
```

## Patterns

```
pattern     = '_' | INT | FLOAT | STR | CHAR | 'true' | 'false'
            | IDENT | IDENT '(' [pattern_list] ')'
            | IDENT '{' [field_pattern_list] '}'
            | pattern '|' pattern
            | pattern '..' pattern
```

## Types

```
type_expr   = IDENT | IDENT '[' type_list ']'
            | '(' [type_list] ')' | '(' [type_list] ')' '->' type_expr
```

## Custom Type Definitions

```
type_def    = 'type' IDENT ['[' type_params ']'] '{' type_variants '}'
type_variants = type_variant { ',' type_variant }
type_variant = IDENT ['(' fields ')']
fields      = field { ',' field }
field       = IDENT ':' type_expr
```

## Impl and Trait

```
impl_block  = 'impl' IDENT ['for' IDENT] '{' { fn_def } '}'
trait_def   = 'trait' IDENT '{' { trait_method } '}'
trait_method = 'fn' IDENT '(' [fn_params] ')' ['->' type_expr]
```

## Tokens

```
IDENT       = [a-zA-Z_][a-zA-Z0-9_]*
INT         = [0-9](_?[0-9])*
FLOAT       = [0-9](_?[0-9])* '.' [0-9](_?[0-9])*
STR         = '"' { any_character | escape_sequence | '{' expr '}' } '"'
CHAR        = '\'' (any_character | escape_sequence) '\''

Keywords:
fn, let, mut, if, else, match, for, while, loop,
break, continue, return, type, trait, impl, pub,
import, as, true, false, nil, in, and, or, not, where

Operators:
+, -, *, /, %, =, ==, !=, <, >, <=, >=,
+=, -=, *=, /=, ->, =>, |>, .., &&, ||, !
```

## Comments

```
line_comment   = '//' { any_character } newline
block_comment  = '/*' { any_character } '*/'
```
