use std::iter;

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

#[test]
/// allowed in wildcard summarizations
fn underscores_and_dots_in_function_names() {
    let func = "function _.hello.world_(int fizz, int buzz) {\nreturn\n}";
    let parsed = parse_exactly_one(func).unwrap();
    let Ast::Function { name, .. } = parsed.ast else { panic!() };

    assert_eq!(name, "_.hello.world_");
}

#[test]
fn import_stmt() {
    let good = "import \"everything/but/the/kitchen/sink\";";
    let bad = "import everything/but/the/kitchen/sink;";

    assert_eq!(
        parse_exactly_one(good).unwrap().ast,
        Ast::Import("everything/but/the/kitchen/sink".to_owned())
    );
    assert!(parse_zero(bad).is_ok());
    assert!(parse_fails_without_semicolon(iter::once(good)));
}

#[test]
fn using_stmt() {
    let src = "using DummyERC20Impl as stake_token;";
    let parsed = parse_exactly_one(src).unwrap();
    assert_eq!(
        parsed.ast,
        Ast::Using {
            contract_name: "DummyERC20Impl".to_owned(),
            spec_name: "stake_token".to_owned()
        }
    );

    let without_semicolon = src.trim_end_matches(';');
    assert!(parse_zero(without_semicolon).is_ok());
}

#[test]
fn use_stmt() {
    let rule = "use rule tuktuk;";
    let rule_with_filtered = "use rule tuktuk filtered { f -> foo(f), g -> bar(g) }";

    let builtin_rule = "use builtin rule blabla;";

    // we don't seem to have any example of "use invariant"s with proofs in EVMVerifier source
    let invariant = "use invariant zamzam;";
    let invariant_with_proof = "use invariant zamzam { preserved { require hello() < world; } }";

    assert_eq!(
        parse_exactly_one(rule).unwrap().ast,
        Ast::UseRule {
            name: "tuktuk".to_owned(),
            filters: None
        }
    );

    assert_eq!(
        parse_exactly_one(rule_with_filtered).unwrap().ast,
        Ast::UseRule {
            name: "tuktuk".to_owned(),
            filters: Some("{ f -> foo(f), g -> bar(g) }".to_owned())
        }
    );

    assert_eq!(
        parse_exactly_one(builtin_rule).unwrap().ast,
        Ast::UseBuiltinRule {
            name: "blabla".to_owned()
        }
    );

    assert_eq!(
        parse_exactly_one(invariant).unwrap().ast,
        Ast::UseInvariant {
            name: "zamzam".to_owned(),
            proof: None
        }
    );

    assert_eq!(
        parse_exactly_one(invariant_with_proof).unwrap().ast,
        Ast::UseInvariant {
            name: "zamzam".to_owned(),
            proof: Some("preserved { require hello() < world; }".to_owned()),
        }
    );

    assert!(parse_fails_without_semicolon([
        rule,
        builtin_rule,
        invariant
    ]));
}

fn parse_fails_without_semicolon(srcs: impl IntoIterator<Item = &'static str>) -> bool {
    srcs.into_iter()
        .map(|src| src.trim_end_matches(';'))
        .all(|without_semicolon| parse_zero(without_semicolon).is_ok())
}
