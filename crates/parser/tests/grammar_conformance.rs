//! Conformance tests for `docs/formal_grammar.md`
//!
//! Each test here pins a claim made by the formal grammar document, so that if the parser's
//! behaviour changes the document is flagged as out of date. Tests are grouped by the section of
//! the document that makes the claim.
//!
//! Only parse-level claims live here. Claims about precedence and associativity are observable
//! through evaluation rather than through parse success, so they're covered by
//! `koto/tests/grammar_conformance.koto` instead.

use koto_parser::Parser;

#[track_caller]
fn accepts(source: &str) {
    if let Err(error) = Parser::parse(source) {
        panic!("Expected the parser to accept:\n{source}\n\nError: {error}");
    }
}

#[track_caller]
fn rejects(source: &str) {
    if Parser::parse(source).is_ok() {
        panic!("Expected the parser to reject:\n{source}");
    }
}

mod comments {
    use super::*;

    #[test]
    fn block_comments_do_not_nest() {
        accepts("#- a -#\n1");
        // The first '-#' ends the comment, leaving 'c -#' as stray text
        rejects("#- a #- b -# c -#\n1");
    }
}

mod numbers {
    use super::*;

    #[test]
    fn only_a_lowercase_e_introduces_an_exponent() {
        accepts("1e5");
        accepts("1.5e3");
        accepts("1e-5");
        rejects("1E5");
        rejects("1.5E3");
    }

    #[test]
    fn a_fraction_needs_at_least_one_digit() {
        accepts("1.5");
        // '1.' lexes as 1 followed by an access
        rejects("x = 1.");
        // The first digit after '.' can't be an underscore
        rejects("x = 1._5");
    }

    #[test]
    fn radix_prefixes_and_digit_separators() {
        accepts("0x1f");
        accepts("0b1010");
        accepts("0o17");
        accepts("1_000");
        accepts("0xdead_beef");
        rejects("0b");
    }
}

mod keywords {
    use super::*;

    #[test]
    fn keywords_are_available_as_map_keys_after_a_dot() {
        accepts("m = {'for': 1}\nm.for");
        accepts("m = {'match': 1}\nm.match");
        accepts("m = {'if': 1}\nm.if");
    }

    #[test]
    fn else_is_matched_before_the_dot_exemption_applies() {
        // Documented quirk: unlike every other keyword, 'else' isn't usable after a '.'
        rejects("m = {'else': 1}\nm.else");
    }

    #[test]
    fn else_if_is_a_single_token_needing_exactly_one_space() {
        accepts("if false\n  1\nelse if true\n  2");
        rejects("if false\n  1\nelse  if true\n  2");
    }

    #[test]
    fn await_and_const_are_reserved() {
        rejects("await");
        rejects("const");
    }
}

mod strings {
    use super::*;

    #[test]
    fn supported_escape_sequences() {
        accepts(r"'\n\r\t'");
        accepts(r"'\\'");
        accepts(r"'\''");
        accepts(r#"'\"'"#);
        accepts(r"'\{}'");
        accepts(r"'\x41'");
        accepts(r"'\u{41}'");
        // \u{...} isn't limited to six digits
        accepts(r"'\u{0000041}'");
    }

    #[test]
    fn a_backslash_before_a_newline_is_a_line_continuation() {
        accepts("'a\\\n   b'");
    }

    #[test]
    fn unsupported_escape_sequences() {
        rejects(r"'\0'");
        rejects(r"'\e'");
        rejects(r"'\$'");
    }

    #[test]
    fn raw_strings() {
        accepts(r"r'a\nb'");
        accepts("r#'it's'#");
    }
}

mod expressions {
    use super::*;

    #[test]
    fn negation_requires_no_whitespace_before_its_operand() {
        accepts("x = 5\n-x");
        rejects("x = 5\ny = - x");
    }

    #[test]
    fn ranges_parse_but_dont_chain_usefully() {
        accepts("1..5");
        accepts("1..=5");
        // Open-started ranges are ordinary expressions, not just index syntax
        accepts("x = ..10");
        accepts("x = ..");
        // Parses as 1..(2..3); rejected later when the range is constructed
        accepts("1..2..3");
    }
}

mod chains {
    use super::*;

    #[test]
    fn a_dot_access_must_be_followed_immediately_by_its_key() {
        accepts("m = {a: 1}\nm.a");
        rejects("m = {a: 1}\nm. a");
    }

    #[test]
    fn null_checks_cannot_be_repeated() {
        accepts("m = {a: null}\nm.a?.b");
        rejects("m = {a: null}\nm.a??");
    }
}

mod calls {
    use super::*;

    #[test]
    fn unpacking_is_postfix_only() {
        accepts("f = |a, b| a\nx = (1, 2)\nf x...");
        rejects("f = |a, b| a\nx = (1, 2)\nf ...x");
    }

    #[test]
    fn paren_free_args_need_commas_even_across_lines() {
        accepts("f = |a, b| a\nf\n  1,\n  2");
        rejects("f = |a, b| a\nf\n  1\n  2");
    }
}

mod primary {
    use super::*;

    #[test]
    fn a_bare_identifier_cannot_carry_a_type_hint() {
        accepts("let x: Number = 42");
        rejects("x: Number = 42");
    }

    #[test]
    fn parenthesized_forms() {
        accepts("()");
        accepts("(1 + 2)");
        accepts("(1,)");
        accepts("(,)");
    }
}

mod functions {
    use super::*;

    #[test]
    fn a_variadic_can_be_the_only_parameter() {
        accepts("f = |xs...| xs");
    }

    #[test]
    fn a_variadic_cannot_have_a_type_hint() {
        rejects("f = |xs: Number...| xs");
    }

    #[test]
    fn defaults_must_come_last() {
        accepts("f = |a, b = 2| a");
        rejects("f = |a = 1, b| a");
    }

    #[test]
    fn any_parameter_form_can_have_a_default() {
        accepts("f = |(a, b) = (1, 2)| a");
        accepts("f = |{a} = {a: 1}| a");
        accepts("f = |_x = 1| 1");
    }

    #[test]
    fn a_trailing_comma_is_allowed() {
        accepts("f = |a, b,| a");
    }

    #[test]
    fn self_cannot_be_declared_as_a_parameter() {
        rejects("f = |self| 1");
    }

    #[test]
    fn map_patterns_take_type_hints_but_tuple_patterns_dont() {
        accepts("f = |{a}: Map| a");
        rejects("f = |(a, b): Tuple| a");
    }
}

mod type_hints {
    use super::*;

    #[test]
    fn nullable_types_are_supported_nested_types_are_not() {
        accepts("let x: Number? = null");
        rejects("let x: List<Number> = []");
    }
}

mod bindings {
    use super::*;

    #[test]
    fn map_patterns_rebind_with_as_and_type_hint_with_colon() {
        accepts("m = {a: 1}\n{a as b} = m");
        accepts("let {a: Number} = {a: 1}");
        // ':' is read as a type hint, so 'b' is parsed as a type
        rejects("m = {a: 1}\n{a: b} = m");
    }

    #[test]
    fn a_string_key_must_be_rebound() {
        accepts("m = {'a b': 1}\n{'a b' as c} = m");
        rejects("m = {'a b': 1}\n{'a b'} = m");
    }

    #[test]
    fn let_accepts_multiple_targets() {
        accepts("let a, b = 1, 2");
        accepts("let a: Number, b: String = 1, 'x'");
    }
}

mod if_expressions {
    use super::*;

    #[test]
    fn the_block_form_takes_no_then() {
        accepts("if true\n  1");
        rejects("if true then\n  1");
    }

    #[test]
    fn the_inline_form_has_no_else_if_chain() {
        accepts("if true then 1 else 2");
        rejects("if false then 1 else if true then 2 else 3");
    }
}

mod match_expressions {
    use super::*;

    #[test]
    fn arms_require_then() {
        accepts("match 1\n  1 then 'one'\n  else 'other'");
        rejects("match 1\n  1\n    'one'");
    }

    #[test]
    fn alternatives_are_separated_by_or() {
        accepts("match 3\n  1 or 2 then 'low'\n  else 'high'");
    }

    #[test]
    fn guards_use_if_between_pattern_and_then() {
        accepts("match 5\n  n if n > 3 then 'big'\n  else 'small'");
    }

    #[test]
    fn patterns_are_not_general_expressions() {
        rejects("y = 2\nmatch 2\n  y + 0 then 'yes'\n  else 'no'");
    }

    #[test]
    fn else_is_an_arm_form_not_a_pattern() {
        accepts("match 1\n  else 'x'");
        rejects("match 1\n  else then 'x'");
    }

    #[test]
    fn ellipsis_patterns_are_only_valid_when_nested() {
        accepts("match (1, 2, 3)\n  (first, rest...) then first\n  else 0");
        rejects("match (1, 2, 3)\n  rest... then rest\n  else 0");
    }
}

mod switch_expressions {
    use super::*;

    #[test]
    fn arms_are_condition_then_body() {
        accepts("x = 1\nswitch\n  x == 1 then 'one'\n  else 'other'");
    }

    #[test]
    fn arms_do_not_use_if() {
        rejects("x = 1\nswitch\n  if x == 1 then 'one'");
    }
}

mod loops {
    use super::*;

    #[test]
    fn loop_bodies_must_be_indented_blocks() {
        accepts("for x in 1..3\n  x");
        accepts("while false\n  1");
        accepts("until true\n  1");
        accepts("loop\n  break");

        rejects("for x in 1..3 x");
        rejects("while false 1");
        rejects("until true 1");
        rejects("loop break");
    }
}

mod error_handling {
    use super::*;

    #[test]
    fn catch_is_required_and_takes_a_binding() {
        accepts("try\n  1\ncatch e\n  2");
        accepts("try\n  1\ncatch _\n  2");
        accepts("try\n  1\ncatch e\n  2\nfinally\n  3");

        rejects("try\n  1");
        rejects("try\n  1\ncatch\n  2");
        rejects("try\n  1\nfinally\n  2");
    }
}

mod modules {
    use super::*;

    #[test]
    fn dotted_paths_require_from() {
        accepts("from string import to_number");
        accepts("import string");
        rejects("import string.to_number");
    }

    #[test]
    fn wildcard_imports_require_from() {
        accepts("from string import *");
        rejects("import *");
    }

    #[test]
    fn import_items_can_be_renamed() {
        accepts("import string as s");
        accepts("from string import to_number as n");
    }

    #[test]
    fn export_accepts_any_assigning_expression() {
        accepts("export a = 1");
        accepts("export { b: 2 }");
        accepts("export\n  c: 3");
    }
}

mod meta_keys {
    use super::*;

    #[test]
    fn test_and_meta_require_a_following_identifier() {
        accepts("@test my_test = || 1");
        accepts("m = {@meta foo: 1}");

        rejects("@test = || 1");
        rejects("@test 'my test' = || 1");
        rejects("m = {@meta: 1}");
    }

    #[test]
    fn operator_meta_keys_take_no_suffix() {
        accepts("m = {@+: |o| o}");
        accepts("m = {@r+: |o| o}");
        accepts("m = {@index: |i| i}");
        accepts("m = {@display: || 'x'}");
    }
}
