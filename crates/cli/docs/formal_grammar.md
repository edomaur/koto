# Koto Formal Grammar (Python-style notation)

This document is a formal grammar for the [Koto](https://koto.dev) programming
language, extracted from the language's implementation:

- Lexer: `crates/lexer/src/lexer.rs`
- Parser (recursive descent): `crates/parser/src/parser.rs`
- AST definitions: `crates/parser/src/node.rs`
- Language guide: `crates/cli/docs/language_guide.md`

Because Koto is implemented as a recursive-descent parser (rather than from a
grammar specification), this document reconstructs the grammar from the parsing
functions and the explicit operator-precedence table.

The notation follows the same conventions as the
[Python Language Reference](https://docs.python.org/3/reference/introduction.html#notation).

---

## Notation

The descriptions of lexical analysis and syntax use a modified Backus–Naur form
(BNF) grammar notation. This uses the following style of definition:

- `name ::= ` — the name of a rule, followed by `::=`, then the definition.
- `|` — separates alternatives; the least-binding operator.
- `*` — zero or more repetitions of the preceding item.
- `+` — one or more repetitions of the preceding item.
- `[ ]` — the enclosed item is optional (zero or one occurrence).
- `( )` — groups items together.
- `indented( )` — the enclosed items must appear on lines indented more deeply
  than the construct that introduces them (see below).
- String literals are enclosed in quotes.
- Whitespace is only meaningful to separate tokens, except where noted.

Uppercase names (`NAME`, `NUMBER`, `STRING`, `NEWLINE`) denote lexical tokens;
they are defined in §1 and used as atoms in §2.

Repetition of the form `X (op X)*` says nothing about associativity; the
associativity of each binary operator is given in §3. Several of Koto's
operators are right-associative even though they are written this way here.

Koto is **indentation-sensitive**: a block is delimited by an increase in
indentation relative to its enclosing context. Where indentation is part of the
syntax it is marked with `indented( )`, since BNF alone cannot express it.

---

## 1. Lexical Grammar

### 1.1 Source and whitespace

```
source ::= token*
token ::= whitespace | NEWLINE | comment | NUMBER | STRING | NAME
        | keyword | operator | punctuation
whitespace ::= (' ' | '\t')+
```

Whitespace is used for indentation and to separate tokens, but is otherwise
ignored. Newlines terminate expressions (a semicolon `;` may be used instead to
place multiple expressions on one line).

A few constructs are sensitive to whitespace between tokens; these are noted
where they appear (`-` as negation in §2.2, `(` as a call in §2.4).

### 1.2 Comments

```
comment ::= line_comment | block_comment
line_comment ::= '#' (any character except newline)*
block_comment ::= '#-' (any character, up to the first '-#')* '-#'
```

Block comments **do not nest**: the first `-#` ends the comment, so
`#- a #- b -# c -#` is a comment followed by the stray text `c -#`.

### 1.3 Identifiers

Identifiers use the Unicode *XID* classification (per `unicode-xid`):

```
NAME ::= XID_Start XID_Continue*
```

A leading `_` produces an *ignored* identifier (`_` or `_name`), used to
discard values:

```
ignored ::= '_' XID_Continue*
```

### 1.4 Keywords

The following words are reserved keywords (they cannot be used as identifiers
unless directly preceded by a `.`):

```
keyword ::= 'as' | 'and' | 'await' | 'break' | 'catch' | 'const' | 'continue'
          | 'debug' | 'else' | 'else if' | 'export' | 'false' | 'finally'
          | 'for' | 'from' | 'if' | 'import' | 'in' | 'let' | 'loop' | 'match'
          | 'not' | 'null' | 'or' | 'return' | 'self' | 'switch' | 'then'
          | 'throw' | 'true' | 'try' | 'until' | 'while' | 'yield'
```

`else if` is recognized as a single keyword, and requires exactly one space
between the two words. `await` and `const` are reserved for future use and
produce an error when used.

The `.`-prefix exemption is applied after `else` is matched, so while
`m.for` and `m.match` are valid map accesses, `m.else` is a syntax error.

### 1.5 Literals

#### Numbers

```
NUMBER ::= decimal | binary | octal | hex

decimal ::= digit decimal_digit* [ '.' digit decimal_digit* ] [ exponent ]

binary ::= '0b' binary_digit+
octal ::= '0o' octal_digit+
hex ::= '0x' hex_digit+

exponent ::= 'e' ['+' | '-'] decimal_digit+

digit ::= '0'..'9'
decimal_digit ::= '0'..'9' | '_'
binary_digit ::= '0' | '1' | '_'
octal_digit ::= '0'..'7' | '_'
hex_digit ::= '0'..'9' | 'a'..'f' | 'A'..'F' | '_'
```

Only a lowercase `e` introduces an exponent; `1E5` is a syntax error.

A fractional part needs at least one digit after the `.`, and that digit can't
be `_`. `1.` is lexed as the number `1` followed by `.`, and `1._5` as `1`
followed by an access of `_5`.

A leading `-` (or `+` via unary operators) is handled at the expression level,
not as part of the numeric literal.

#### Strings

Strings may use single or double quotes; the two forms are equivalent.

```
STRING ::= quoted_string | raw_string

quoted_string ::= ('\'' | '"') string_char* ('\'' | '"')

raw_string ::= 'r' '#'* delimiter (any character)* delimiter
delimiter ::= '\'' | '"'     # closing delimiter matches the opening quote and '#' count
```

In a quoted string, `{` begins an interpolated expression:

```
string_char ::= escaped_char | text_char | interpolation

text_char ::= any character except the closing quote, '{', or '\'

interpolation ::= '{' expression [ ':' format_options ] '}'

format_options ::= [ [ fill ] alignment ] [ '0' ] [ min_width ]
                   [ '.' precision ] [ representation ]
fill ::= any single grapheme cluster      # must be followed by an alignment,
                                          # unless it ends the format options
alignment ::= '<' | '^' | '>'
min_width ::= digit+
precision ::= digit+
representation ::= '?' | 'b' | 'o' | 'x' | 'X' | 'e' | 'E'

escaped_char ::= '\' ('n' | 'r' | 't' | '\' | '\'' | '"' | '{')
               | '\' 'x' hex hex             # value must be <= 0x7F
               | '\' 'u' '{' hex* '}'        # must form a valid code point
               | '\' NEWLINE                 # line continuation

hex ::= '0'..'9' | 'a'..'f' | 'A'..'F'
```

`\` followed by a newline is a line continuation: the newline and the
following line's leading whitespace are both discarded.

Escape sequences other than those listed above are errors; in particular there
is no `\0` and no `\e`. `\u{...}` accepts any number of hex digits as long as
the result is a valid code point.

Raw strings ignore escape sequences and interpolation. The `r` prefix may be
followed by up to 255 `#` characters, in which case the closing delimiter must
be the matching quote followed by the same number of `#`.

Interpolated values can carry format options (e.g. `'{x:.2}'`), following the
same syntax as Rust's format specifiers (width, alignment, precision, fill).
See `crates/parser/src/string_format_options.rs`.

### 1.6 Operators and punctuation

```
operator ::= arithmetic | comparison | assignment | '->' | '?' | '...'
           | '..' | '..='

arithmetic ::= '+' | '-' | '*' | '/' | '%' | '^'
comparison ::= '==' | '!=' | '<' | '<=' | '>' | '>='
assignment ::= '=' | '+=' | '-=' | '*=' | '/=' | '%=' | '^='

punctuation ::= '(' | ')' | '[' | ']' | '{' | '}' | '|'
              | '@' | '.' | ',' | ':' | ';'
```

`and`, `or`, and `not` are keywords (§1.4) rather than operator tokens, but they
behave as operators in the grammar (§2.2) and appear in the precedence table
(§3). `|` delimits function arguments (§2.7); it is not a binary operator.

---

## 2. Syntactic Grammar

### 2.1 Program and blocks

```
program ::= line*
line ::= expressions [ ';' | NEWLINE ]

indented_block ::= indented( line+ )
```

The top-level program is a sequence of lines at indentation 0. An indented
block (used after `if`, `for`, function definitions, etc.) consists of lines
indented more deeply than the construct that introduces it.

Wherever the rules below say `indented_block`, an indented block is the *only*
accepted form — there is no single-line variant.

### 2.2 Expressions

```
expressions ::= expression (',' expression)*     # a bare tuple when there's more than one

expression ::= operator_expression [ range_suffix ]
             | range_suffix                      # range with no start

range_suffix ::= ('..' | '..=') [ expression ]

operator_expression ::= targets '=' expressions
                      | operand (binary_op operand)*

binary_op ::= '->'                                        # pipe
            | '+=' | '-=' | '*=' | '/=' | '%=' | '^='
            | 'or' | 'and'
            | '==' | '!=' | '<' | '<=' | '>' | '>='
            | '+' | '-' | '*' | '/' | '%' | '^'

operand ::= 'not' expression
          | '-' postfix               # no whitespace between '-' and the operand
          | postfix
```

Precedence and associativity for `binary_op` are given in §3; the parser uses
precedence climbing over a single operator table rather than a cascade of
rules.

Two prefix forms deserve attention:

- **`not` takes a whole expression as its operand**, so it binds less tightly
  than every binary operator and less tightly than `=`:
  `not a and b` is `not (a and b)`, and `not a = b` is `not (a = b)`.
- **`-` takes only a postfix expression**, and there must be no whitespace
  before its operand. `- x` is not a negation, and `-2 ^ 2` is `(-2) ^ 2`.

A range suffix is applied to a fully parsed expression, so it binds less tightly
than every binary operator: `1 + 1 .. 2 + 2` is `(1 + 1) .. (2 + 2)`. Ranges
don't usefully chain — `1 .. 2 .. 3` parses as `1 .. (2 .. 3)` and is rejected
when the range is constructed.

### 2.3 Postfix (chains)

A *chain* is a sequence of accesses, indexing, calls, and null-checks applied to
a primary expression.

```
postfix ::= primary chain_suffix*

chain_suffix ::= '.' (NAME | STRING)     # member access
               | '[' index_expression ']'
               | call
               | '?'                     # short-circuit null check
```

The `?` suffix short-circuits a chain: if the value is `null`, the rest of the
chain evaluates to `null`. Two `?` in a row is an error.

A `.` access may appear on a following, more deeply indented line. The key must
follow the `.` immediately — `x. foo` is an error.

### 2.4 Calls

Parentheses around call arguments are optional in many positions.

```
call ::= '(' [ arguments ] ')'            # no whitespace before '('
       | space_separated_args

arguments ::= argument (',' argument)* [ ',' ]

argument ::= expression [ '...' ]         # '...' unpacks an iterable

space_separated_args ::= argument (',' argument)*
```

For example `f a, b` is equivalent to `f(a, b)`.

Unpacking is postfix only: `f xs...` is valid, `f ...xs` is not.

Whitespace before `(` matters: `f(x)` is a call, whereas `f (x)` is a
space-separated call whose single argument is the parenthesized expression `x`.

Space-separated arguments are parsed with a minimum precedence that excludes the
pipe operator, so `f g -> x` is `(f g) -> x` rather than `f (g -> x)`.
Arguments must be comma-separated even when spread over several lines:

```koto
do_something = |a, b| a + b

result = do_something
  1,
  2

print! result
check! 3
```

### 2.5 Primary expressions

```
primary ::= 'null'
          | 'true'
          | 'false'
          | NUMBER
          | STRING
          | NAME
          | ignored
          | 'self'
          | meta_assignment
          | parenthesized
          | list
          | map
          | function
          | if_expression
          | match_expression
          | switch_expression
          | loop_expression
          | for_loop
          | while_loop
          | until_loop
          | break_expression
          | continue_expression
          | return_expression
          | throw_expression
          | debug_expression
          | yield_expression
          | import
          | export
          | try_expression
          | let_expression

break_expression ::= 'break' [ expressions ]     # optional loop result
continue_expression ::= 'continue'
return_expression ::= 'return' [ expressions ]
throw_expression ::= 'throw' expression
debug_expression ::= 'debug' expressions
yield_expression ::= 'yield' expressions
```

A bare `NAME` used as an expression cannot carry a type hint; type hints only
appear in bindings (§2.8).

A meta key at expression position must be the target of an assignment or of a
map block entry:

```
meta_assignment ::= meta_key '=' expressions
                  | meta_key ':' map_block_value
```

### 2.6 Collections and ranges

```
parenthesized ::= '(' ')'                                       # empty tuple
                | '(' expression ')'                            # grouping
                | '(' expression (',' expression)* [ ',' ] ')'  # tuple
                | '(' ',' ')'                                   # 1-tuple holding null

list ::= '[' [ expression (',' expression)* [ ','] ] ']'

map ::= map_with_braces | map_block

map_with_braces ::= '{' [ map_entry (',' map_entry)* [ ','] ] '}'
map_entry ::= map_key ':' expression
            | NAME                                 # shorthand; value taken from scope
            | (NAME | STRING) 'as' binding_target  # only valid on the LHS of an assignment
map_key ::= NAME | STRING | meta_key

map_block ::= indented( map_block_entry+ )   # all entries share one indentation level
map_block_entry ::= map_key ':' map_block_value
map_block_value ::= expressions | indented_block

range ::= expression '..' [ expression ]    # exclusive
        | expression '..=' [ expression ]   # inclusive
        | '..' expression                   # no start
        | '..=' expression                  # no start, inclusive
        | '..'                              # full range (slicing)

index_expression ::= expression
```

`range` restates the `range_suffix` forms of §2.2 in one place; all of them are
ordinary expressions, so `x = ..10` is valid outside an index.

`(expr)` is a grouping, not a one-element tuple; `(expr,)` is the one-element
tuple. `()` is the empty tuple. A tuple can also be written without parentheses
wherever `expressions` is accepted, e.g. `x = 1, 2`.

Only a `NAME` key can use the valueless shorthand; string and meta keys always
need a value.

Map literals may be written either with `{ }` braces (comma-separated entries)
or as an indented *map block*, where keys and values appear on their own lines:

```koto
foo =
  key1: 'a'
  key2: 'b'

print! foo.key2
check! b
```

### 2.7 Functions

```
function ::= '|' [ parameters ] '|' [ '->' type ] body

parameters ::= parameter (',' parameter)* [ ',' ]

parameter ::= NAME [ ':' type ] [ '=' expression ]
            | NAME '...'                            # variadic
            | ignored [ ':' type ] [ '=' expression ]
            | tuple_pattern [ '=' expression ]      # (a, b) — nested unpacking
            | map_pattern [ ':' type ] [ '=' expression ]

tuple_pattern ::= '(' [ nested_parameter (',' nested_parameter)* ] ')'
nested_parameter ::= NAME | ignored | tuple_pattern | map_pattern

body ::= expressions | indented_block
```

Constraints that the grammar can't express:

- A variadic parameter must be last, and can't carry a type hint.
- Once a parameter has a default value, every parameter after it must have one
  too.
- A `tuple_pattern` can't carry a type hint, unlike a `map_pattern`.
- `self` can't be declared as a parameter; it's always implicitly available.

A function whose body contains a `yield` expression is a generator.

### 2.8 Type hints

```
type ::= NAME [ '?' ]                 # '?' marks the type as nullable
```

Nested/generic types are not currently supported by the parser; `List<Number>`
produces a dedicated error.

Type hints are only permitted on bindings: `let` targets, function parameters,
`for` arguments, `catch` arguments, match patterns, and map-pattern entries.

### 2.9 Assignment and bindings

```
targets ::= target (',' target)*        # multiple assignment

target ::= NAME
         | ignored
         | postfix                      # indexed / member assignment
         | map_pattern
         | map_with_braces
         | meta_key                     # export/meta assignment

binding ::= NAME [ ':' type ]
          | ignored [ ':' type ]
          | map_pattern

binding_target ::= (NAME | ignored) [ ':' type ]

map_pattern ::= '{' [ map_pattern_entry (',' map_pattern_entry)* [ ',' ] ] '}'
                  [ ':' type ]
map_pattern_entry ::= NAME [ ':' type ]
                    | NAME 'as' binding_target
                    | STRING 'as' binding_target
```

In a map pattern, `:` introduces a **type hint**; rebinding a key to a different
identifier uses `as`. So `{a as b} = m` binds `m.a` to `b`, while `{a: b} = m`
is an error (`b` is read as a type). A `STRING` key must always be rebound with
`as`, since it isn't a valid identifier.

`let` introduces one or more bindings, each with an optional type check:

```
let_expression ::= 'let' binding (',' binding)* '=' expressions
```

### 2.10 Control flow

#### `if`

`if` has two distinct forms: an inline form using `then`, and a block form that
has no `then` at all.

```
if_expression ::= 'if' expression 'then' expressions [ 'else' expressions ]
                | 'if' expression indented_block
                    ('else if' expression indented_block)*
                    [ 'else' indented_block ]
```

The inline form has no `else if` chain. Note that `else if` is a single token
(§1.4), so the block form can't be written as `else` followed by a nested `if`
on the same line. `if` is an expression: it evaluates to the value of the chosen
branch.

#### `match`

```
match_expression ::= 'match' expressions indented( match_arm+ )

match_arm ::= match_patterns ('or' match_patterns)* [ 'if' expression ]
                'then' arm_body
            | 'else' arm_body

arm_body ::= expressions | indented_block

match_patterns ::= match_pattern (',' match_pattern)*

match_pattern ::= 'null' | 'true' | 'false' | ['-'] NUMBER | STRING
                | NAME [ ':' type ]
                | NAME chain_suffix+
                | ignored [ ':' type ]
                | '(' [ nested_pattern (',' nested_pattern)* ] ')'
                | map_pattern

nested_pattern ::= match_pattern
                 | NAME '...'      # only inside a parenthesized pattern
                 | '...'           # only inside a parenthesized pattern
```

Patterns are a restricted sublanguage, not general expressions: `y + 0` is not a
valid pattern.

Each `or`-separated alternative must supply the same number of comma-separated
patterns. An `else` arm takes no pattern and no guard, and must be the final arm.

#### `switch`

```
switch_expression ::= 'switch' indented( switch_arm+ )
switch_arm ::= expression 'then' arm_body
             | 'else' arm_body
```

An `else` arm must be the final arm.

#### Loops

```
loop_expression ::= 'loop' indented_block
for_loop ::= 'for' for_args 'in' expression indented_block
for_args ::= binding (',' binding)*
while_loop ::= 'while' expression indented_block
until_loop ::= 'until' expression indented_block
```

All four loop bodies must be indented blocks; there is no inline form.

### 2.11 Error handling

```
try_expression ::= 'try' indented_block
                     ('catch' binding indented_block)+
                     [ 'finally' indented_block ]
```

At least one `catch` is required, and each `catch` requires a binding for the
caught error.

### 2.12 Modules

```
import ::= 'import' import_item (',' import_item)*
         | 'from' from_path 'import' (import_items | '*')

from_path ::= (NAME | STRING) ('.' (NAME | STRING))*

import_items ::= import_item (',' import_item)*
import_item ::= (NAME | STRING) [ 'as' NAME ]

export ::= 'export' expressions
```

Dotted paths are only available via `from`: `import string.to_number` is an
error that suggests using `from` instead. A wildcard `*` likewise requires a
`from`.

`export` accepts any expression that assigns, so all of these are valid:

```koto
export a = 1
export { b: 2, c: 3 }
export
  d: 4
  e: 5

print! a + b + c + d + e
check! 15
```

### 2.13 Meta keys

Meta keys (`@name`) define object behavior and metadata.

```
meta_key ::= '@' meta_operator
           | '@' 'test' NAME
           | '@' 'meta' NAME

meta_operator ::= '+' | '-' | '*' | '/' | '%' | '^'
                | 'r+' | 'r-' | 'r*' | 'r/' | 'r%' | 'r^'
                | '+=' | '-=' | '*=' | '/=' | '%=' | '^='
                | '<' | '<=' | '>' | '>=' | '==' | '!='
                | 'index' | 'index_assign'
                | 'access' | 'access_assign'
                | 'debug' | 'display' | 'iterator' | 'next' | 'next_back'
                | 'negate' | 'size' | 'type' | 'base' | 'call'
                | 'pre_test' | 'post_test' | 'main'
```

`@test` and `@meta` both require a following identifier, which can't be a
string: `@test my_test` is valid, `@test` and `@test 'my test'` are not.

---

## 3. Operator Precedence

Operators are listed from lowest to highest precedence. Associativity is as
implemented in the parser's precedence table (`operator_precedence()`).

| Precedence  | Operators                            | Associativity   |
| ----------- | ------------------------------------ | --------------- |
| 1 (lowest)  | `not` (prefix)                       | —               |
| 2           | `=` (assignment)                     | right           |
| 3           | `..` `..=` (range)                   | non-associative |
| 4           | `->` (pipe)                          | left            |
| 5           | `+=` `-=` `*=` `/=` `%=` `^=`        | right           |
| 6           | `or`                                 | left            |
| 7           | `and`                                | left            |
| 8           | `==` `!=`                            | right           |
| 9           | `<` `<=` `>` `>=`                    | right           |
| 10          | `+` `-`                              | left            |
| 11          | `*` `/` `%`                          | left            |
| 12          | `^` (power)                          | left            |
| 13 (highest)| `-` (negation)                       | —               |

Notes on the entries that differ from what most languages do:

- `not` isn't a tight-binding unary operator: it consumes the whole expression
  that follows it, which puts it below assignment.
- `..` and `..=` are applied to an already-parsed expression rather than
  participating in precedence climbing, which puts them below every binary
  operator. `1 .. 2 .. 3` nests to the right and is rejected when the range is
  constructed, so they're effectively non-associative.
- `^` is **left**-associative, so `2 ^ 3 ^ 2` is `64`, not `512`.
- `==`/`!=` and the relational operators are right-associative.
- Negation binds tighter than `^`, so `-2 ^ 2` is `4`.

Postfix operations (member access, indexing, calls, `?`) bind tighter than all
of the above.

Paren-free call arguments are parsed with a minimum precedence that excludes the
pipe operator but admits everything from the compound assignments upwards. So
`f g -> x` is `(f g) -> x`, while `f x += 1` passes `x += 1` as the argument.
(Assignment is checked separately from the precedence table, so `f x = 1` also
passes `x = 1` as the argument.)

---

## Notes

- This grammar is a faithful reconstruction of the Koto parser's behavior, but
  the parser is the authoritative definition. Discrepancies should be resolved
  in favor of the source in `crates/parser/src/parser.rs`.
- Indentation rules (block boundaries, expression continuation) are only
  partially captured by BNF; see the parser's `ExpressionContext` /
  `Indentation` handling for the exact rules.
