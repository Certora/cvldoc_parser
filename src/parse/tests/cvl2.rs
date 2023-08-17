use super::*;
use indoc::formatdoc;

#[rustfmt::skip]
#[test]
/// we currently allow any code inside a `methods` block
fn methods_entries_need_function_and_semicolon() {
    let cvl2_style = "function balanceOf(address) returns(uint) envfree;";
    let without_semicolon = "function balanceOf(address) returns(uint) envfree";
    let without_function_or_semicolon = "balanceOf(address) returns(uint) envfree";

    for function_decl in [cvl2_style, without_semicolon, without_function_or_semicolon] {
        let block = formatdoc!("methods {{
            {function_decl}
        }}");

        let parsed = parse_exactly_one(&block).unwrap();
        assert_matches!(parsed.ast, Ast::Methods { block } if block == function_decl);
    }
}

#[test]
fn invariant_requires_semicolon() {
    let without_semicolon = "invariant validityOfTokens() true";
    assert!(parse_zero(without_semicolon).is_ok());

    let with_semicolon = "invariant validityOfTokens() true;";
    let parsed = parse_exactly_one(with_semicolon).unwrap();
    let Ast::Invariant { name, invariant, .. } = parsed.ast else { panic!() };
    assert_eq!(name, "validityOfTokens");
    assert_eq!(invariant, "true");
}

#[test]
fn rule_keyword_is_mandatory() {
    let without_kw = "onlyOwnerCanDecrease() { }";
    assert!(parse_zero(without_kw).is_ok());

    let with_kw = "rule onlyOwnerCanDecrease() { }";
    let parsed = parse_exactly_one(with_kw).unwrap();
    let Ast::Rule { name, .. } = parsed.ast else { panic!() };
    assert_eq!(name, "onlyOwnerCanDecrease")
}