use std::iter;

use super::*;
use indoc::formatdoc;
use itertools::Itertools;

#[rustfmt::skip]
#[test]
/// we currently allow any code inside a `methods` block
fn methods_entries_need_function_and_semicolon() {
    let cvl2_style = "function balanceOf(address) returns(uint) envfree;";
    let without_semicolon = "function balanceOf(address) returns(uint) envfree";
    let without_function_or_semicolon = "balanceOf(address) returns(uint) envfree";

    for function_decl in [cvl2_style, without_semicolon, without_function_or_semicolon] {
        let block = formatdoc! {
            "methods {{
                {function_decl}
            }}
        "};

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

#[test]
fn hook_sload1() {
    let src = indoc! {"
        hook Sload uint256 value a STORAGE {
            require value == aGhost;
        }
    "};
    let parsed = parse_exactly_one(src).unwrap();
    let Ast::HookSload { name, ty, slot_pattern, block } = parsed.ast else { panic!() };

    assert_eq!(name, "value");
    assert_eq!(ty, "uint256");
    assert_eq!(slot_pattern, "a");
    assert_eq!(block, "require value == aGhost;");
}

#[test]
fn hook_sload2() {
    let src = "hook Sload uint256 imp (slot 50801780122331352337026042894847907698553222651959119521779622085092237899971/*0x7050c9e0f4ca769c69bd3a8ef740bc37934f8e2c036e5a723fd8ee048ed3f8c3*/) STORAGE {}";
    let parsed = parse_exactly_one(src).unwrap();
    let Ast::HookSload { name, ty, slot_pattern, .. } = parsed.ast else { panic!() };

    assert_eq!(name, "imp");
    assert_eq!(ty, "uint256");
    assert_eq!(slot_pattern, "(slot 50801780122331352337026042894847907698553222651959119521779622085092237899971/*0x7050c9e0f4ca769c69bd3a8ef740bc37934f8e2c036e5a723fd8ee048ed3f8c3*/)");
}

#[test]
fn hook_sload3() {
    let src = indoc! {"
        hook Sload uint value my_mapping_array[INDEX uint idx][KEY uint k] STORAGE {
            havoc bear assuming bear@new(idx) == value;
        }
    "};
    let parsed = parse_exactly_one(src).unwrap();
    let Ast::HookSload { name, ty, slot_pattern, .. } = parsed.ast else { panic!() };

    assert_eq!(name, "value");
    assert_eq!(ty, "uint");
    assert_eq!(slot_pattern, "my_mapping_array[INDEX uint idx][KEY uint k]");
}

#[test]
fn hook_sload4() {
    let src = indoc! {"
        hook Sload uint owner _holderTokens[KEY address k].(offset 0).(offset 0 /* the array */)[INDEX uint i] STORAGE {
            require holderAndArrayIndexToToken(k,i) == owner;
        }
    "};
    let parsed = parse_exactly_one(src).unwrap();
    let Ast::HookSload { name, ty, slot_pattern, block } = parsed.ast else { panic!() };

    assert_eq!(name, "owner");
    assert_eq!(ty, "uint");
    assert_eq!(
        slot_pattern,
        "_holderTokens[KEY address k].(offset 0).(offset 0 /* the array */)[INDEX uint i]"
    );
    assert_eq!(block, "require holderAndArrayIndexToToken(k,i) == owner;");
}

#[test]
fn hook_sstore1() {
    let src = indoc! {"
        hook Sstore _list .(offset 0)[INDEX uint256 index] bytes32 newValue (bytes32 oldValue) STORAGE { }
    "};
    let parsed = parse_exactly_one(src).unwrap();
    let Ast::HookSstore { name_new, name_old, ty, slot_pattern, .. } = parsed.ast else { panic!() };

    assert_eq!(name_new, "newValue");
    assert_eq!(name_old.as_deref(), Some("oldValue"));
    assert_eq!(ty, "bytes32");
    assert_eq!(slot_pattern, "_list .(offset 0)[INDEX uint256 index]");
}

#[test]
fn hook_sstore2() {
    let src = indoc! {"
        hook Sstore _x uint value STORAGE {
            xmap[0] = value;
        }
    "};
    let parsed = parse_exactly_one(src).unwrap();
    let Ast::HookSstore { name_new, name_old, ty, slot_pattern, block } = parsed.ast else { panic!() };

    assert_eq!(name_new, "value");
    assert!(name_old.is_none());
    assert_eq!(ty, "uint");
    assert_eq!(slot_pattern, "_x");
    assert_eq!(block, "xmap[0] = value;");
}

#[test]
fn hook_sstore3() {
    let src = indoc! {"
        hook Sstore g[KEY bytes b][INDEX uint256 j][INDEX uint256 k] uint v STORAGE {
            boop[j] = boop[j] + 1;
        }
    "};
    let parsed = parse_exactly_one(src).unwrap();
    let Ast::HookSstore { name_new, name_old, ty, slot_pattern, block } = parsed.ast else { panic!() };

    assert_eq!(name_new, "v");
    assert!(name_old.is_none());
    assert_eq!(ty, "uint");
    assert_eq!(
        slot_pattern,
        "g[KEY bytes b][INDEX uint256 j][INDEX uint256 k]"
    );
    assert_eq!(block, "boop[j] = boop[j] + 1;");
}

#[test]
fn hook_create() {
    let src = indoc! {"
        hook Create (address createdAddress) { }
    "};
    let parsed = parse_exactly_one(src).unwrap();
    let Ast::HookCreate { name, ty, block } = parsed.ast else { panic!() };

    assert_eq!(name, "createdAddress");
    assert_eq!(ty, "address");
    assert!(block.is_empty());
}

#[test]
fn hook_opcode1() {
    let src = indoc! {"
        hook EXTCODESIZE(address addr) uint v {
            someUint = v;
        };
    "};
    let parsed = parse_exactly_one(src).unwrap();
    let Ast::HookOpcode { opcode, params, returned_value, block } = parsed.ast else { panic!() };

    assert_eq!(opcode, "EXTCODESIZE");
    let param = params.into_iter().exactly_one().unwrap();
    let (param_ty, param_name) = param;
    assert_eq!(param_ty, "address");
    assert_eq!(param_name, "addr");
    let (returned_ty, returned_name) = returned_value.unwrap();
    assert_eq!(returned_ty, "uint");
    assert_eq!(returned_name, "v");
    assert_eq!(block, "someUint = v;");
}

#[test]
fn hook_opcode2() {
    let src = indoc! {"
        hook GASPRICE uint v {
            someUint = v;
        }
    "};
    let parsed = parse_exactly_one(src).unwrap();
    let Ast::HookOpcode { opcode, params, returned_value, block } = parsed.ast else { panic!() };

    assert_eq!(opcode, "GASPRICE");
    assert!(params.is_empty());
    let (returned_ty, returned_name) = returned_value.unwrap();
    assert_eq!(returned_ty, "uint");
    assert_eq!(returned_name, "v");
    assert_eq!(block, "someUint = v;");
}
