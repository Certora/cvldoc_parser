use crate::parse::{cvl_parser, decl_parser};

use super::*;
use indoc::formatdoc;
use itertools::Itertools;
use std::iter;

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
    let Ast::Invariant {
        name, invariant, ..
    } = parsed.ast
    else {
        panic!()
    };
    assert_eq!(name, "validityOfTokens");
    assert_eq!(invariant, "true;");
}

#[test]
fn rule_keyword_is_mandatory() {
    let without_kw = "onlyOwnerCanDecrease() { }";
    assert!(parse_zero(without_kw).is_ok());

    let with_kw = "rule onlyOwnerCanDecrease() { }";
    let parsed = parse_exactly_one(with_kw).unwrap();
    let Ast::Rule { name, .. } = parsed.ast else {
        panic!()
    };
    assert_eq!(name, "onlyOwnerCanDecrease")
}

#[test]
/// allowed in wildcard summarizations
fn underscores_and_dots_in_function_names() {
    let func = "function _.hello.world_(int fizz, int buzz) {\nreturn\n}";
    let parsed = parse_exactly_one(func).unwrap();
    let Ast::Function { name, .. } = parsed.ast else {
        panic!()
    };

    assert_eq!(name, "_.hello.world_");
}

#[test]
fn import_stmt() {
    let good = "import \"everything/but/the/kitchen/sink\";";
    let bad = "import everything/but/the/kitchen/sink;";

    assert_eq!(
        parse_exactly_one(good).unwrap().ast,
        Ast::Import {
            imported: "everything/but/the/kitchen/sink".to_owned()
        }
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
    let Ast::HookSload {
        loaded,
        slot_pattern,
        block,
    } = parsed.ast
    else {
        panic!()
    };

    assert_eq!(loaded.name, "value");
    assert_eq!(loaded.ty, "uint256");
    assert_eq!(slot_pattern, "a");
    assert_eq!(block, "require value == aGhost;");
}

#[test]
fn hook_sload2() {
    let src = "hook Sload uint256 imp (slot 50801780122331352337026042894847907698553222651959119521779622085092237899971/*0x7050c9e0f4ca769c69bd3a8ef740bc37934f8e2c036e5a723fd8ee048ed3f8c3*/) STORAGE {}";
    let parsed = parse_exactly_one(src).unwrap();
    let Ast::HookSload {
        loaded,
        slot_pattern,
        ..
    } = parsed.ast
    else {
        panic!()
    };

    assert_eq!(loaded.name, "imp");
    assert_eq!(loaded.ty, "uint256");
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
    let Ast::HookSload {
        loaded,
        slot_pattern,
        ..
    } = parsed.ast
    else {
        panic!()
    };

    assert_eq!(loaded.name, "value");
    assert_eq!(loaded.ty, "uint");
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
    let Ast::HookSload {
        loaded,
        slot_pattern,
        block,
    } = parsed.ast
    else {
        panic!()
    };

    assert_eq!(loaded.name, "owner");
    assert_eq!(loaded.ty, "uint");
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
    let Ast::HookSstore {
        stored,
        old,
        slot_pattern,
        ..
    } = parsed.ast
    else {
        panic!()
    };

    assert_eq!(stored.name, "newValue");
    let old = old.unwrap();
    assert_eq!(old.name, "oldValue");
    assert_eq!(old.ty, "bytes32");
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
    let Ast::HookSstore {
        stored,
        old,
        slot_pattern,
        block,
    } = parsed.ast
    else {
        panic!()
    };

    assert_eq!(stored.name, "value");
    assert!(old.is_none());
    assert_eq!(stored.ty, "uint");
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
    let Ast::HookSstore {
        stored,
        old,
        slot_pattern,
        block,
    } = parsed.ast
    else {
        panic!()
    };

    assert_eq!(stored.name, "v");
    assert!(old.is_none());
    assert_eq!(stored.ty, "uint");
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
    let Ast::HookCreate { created, block } = parsed.ast else {
        panic!()
    };

    assert_eq!(created.name, "createdAddress");
    assert_eq!(created.ty, "address");
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
    let Ast::HookOpcode {
        opcode,
        params,
        returns,
        block,
    } = parsed.ast
    else {
        panic!()
    };

    assert_eq!(opcode, "EXTCODESIZE");
    let param = params.into_iter().exactly_one().unwrap();
    assert_eq!(param.ty, "address");
    assert_eq!(param.name, "addr");
    let returns = returns.unwrap();
    assert_eq!(returns.ty, "uint");
    assert_eq!(returns.name, "v");
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
    let Ast::HookOpcode {
        opcode,
        params,
        returns,
        block,
    } = parsed.ast
    else {
        panic!()
    };

    assert_eq!(opcode, "GASPRICE");
    assert!(params.is_empty());
    let returns = returns.unwrap();
    assert_eq!(returns.ty, "uint");
    assert_eq!(returns.name, "v");
    assert_eq!(block, "someUint = v;");
}

#[test]
fn invariant_span_is_correct() {
    let src = indoc! {"
        /**
         * @title totalSupply_vs_balance
         * @notice The total supply of the system si zero if and only if the balanceof the system is zero
         * the variant has no parameters
        */
        invariant totalSupply_vs_balance()
            totalSupply() == 0 <=> underlying.balanceOf(currentContract) == 0
            {
                preserved with(env e) {
                    require e.msg.sender != currentContract;
                }
            }
    "};

    let parsed = parse_exactly_one(src).unwrap();
    let Ast::Invariant {
        invariant,
        filters,
        proof,
        ..
    } = parsed.ast
    else {
        panic!()
    };
    assert_eq!(
        invariant,
        "totalSupply() == 0 <=> underlying.balanceOf(currentContract) == 0"
    );

    assert!(filters.is_none());

    assert_eq!(
        proof.unwrap(),
        "preserved with(env e) {\n            require e.msg.sender != currentContract;\n        }"
    );
}

#[test]
fn invariant_span_is_correct2() {
    let src = indoc! {"
        /**
         * @title totalSupply_LE_balance
         * @notice invariant to assure that the total supply is always under the balance amount.
         *  the variant has no parameters.
         * @dev assume currentContract is initiated.
         */
        invariant totalSupply_LE_balance()
            totalSupply() <= underlying.balanceOf(currentContract)
            {
                preserved with(env e) {
                    require e.msg.sender != currentContract;
                }
            }
    "};

    let parsed = parse_exactly_one(src).unwrap();
    let Ast::Invariant {
        invariant,
        filters,
        proof,
        ..
    } = parsed.ast
    else {
        panic!()
    };
    assert_eq!(
        invariant,
        "totalSupply() <= underlying.balanceOf(currentContract)"
    );

    assert!(filters.is_none());

    assert_eq!(
        proof.unwrap(),
        "preserved with(env e) {\n            require e.msg.sender != currentContract;\n        }"
    );
}

#[test]
fn invariant_span_is_correct3() {
    let src = indoc! {r#"
        /// A contract must only ever be in an initializing state while in the middle
        /// of a transaction execution.
        invariant notInitializing()
            !initializing();
        
        
        //////////////////////////////////////////////////////////////////////////////
        //// Rules                                 /////////////////////////////////////
        //////////////////////////////////////////////////////////////////////////////
        
        /// @title Only initialized once
        /// @notice An initializable contract with a function that inherits the
        ///         initializer modifier must be initializable only once
        rule initOnce() {
            uint256 val; uint256 a; uint256 b;
        
            require isInitialized();
            initialize@withrevert(val, a, b);
            assert lastReverted, "contract must only be initialized once";
        }
    "#};

    let parsed = Builder::new(src).build().unwrap();

    let [invariant, freeform, rule] = parsed.as_slice() else {
        let len = parsed.len();
        panic!("expected exactly 3 elements but got {len}")
    };
    let Ast::Invariant { .. } = invariant.ast else {
        panic!()
    };
    let Ast::FreeFormComment { .. } = freeform.ast else {
        panic!()
    };
    let Ast::Rule { .. } = rule.ast else { panic!() };
}

/// as of version 2.0, we no longer parse unexpected tags.
/// any substrings of the form `@foo` where `foo` is not one of the tags
/// defined in [crate::TagKind], will concatenate to the description of the previous tag.
#[test]
fn unrecognized_tags() {
    let src = indoc! {"
        /// @illegal this tag does not exist
        /// @dev this tag does exist
        /// @another_illegal this tag does not exist
        ///      @still_illegal whitespace should be trimmed
        /// @formula hello@withrevert(world)
        function foo(int bar) { }
    "};

    let parsed = Builder::new(src)
        .build()
        .expect("illegal tags should still parse");
    let parsed = parsed.into_iter().exactly_one().unwrap();

    let [tag1, tag2, tag3] = parsed.doc.as_slice() else {
        panic!("should parse to exactly 3 tags")
    };

    assert_matches!(tag1.kind, TagKind::Notice);
    assert_eq!(tag1.description, "@illegal this tag does not exist");

    assert_matches!(tag2.kind, TagKind::Dev);
    assert_eq!(tag2.description, "this tag does exist\n@another_illegal this tag does not exist\n@still_illegal whitespace should be trimmed");

    assert_matches!(tag3.kind, TagKind::Formula);
    assert_eq!(tag3.description, "hello@withrevert(world)"); // @withrevert should not parse to a new tag
}

#[test]
fn persistent_ghosts() {
    let src = indoc! {"
        persistent ghost int pers;
        ghost            int non_pers;
    "};

    let parsed = Builder::new(src).build().unwrap();

    let check_ghost = |element: &CvlElement, expected_name, expected_persistent| match &element.ast
    {
        Ast::GhostMapping {
            persistent, name, ..
        } => {
            assert_eq!(name, expected_name);
            assert_eq!(*persistent, expected_persistent);
        }

        ast => panic!("got unexpected ast: ${ast:?}"),
    };

    assert_eq!(parsed.len(), 2);

    check_ghost(&parsed[0], "pers", true);
    check_ghost(&parsed[1], "non_pers", false);
}

#[test]
/// this shouldn't trip up the definition end detection:
/// the semicolon should be detected even if it's not whitespace-separated from the previous token.
fn definition_does_not_stop() {
    let src = indoc! {"
        methods {
            function x() external returns uint envfree;
            function i() external returns int envfree;
        }
        
        // myInvariant: run0 pass run1 fail
        
        definition addBv(uint x, uint y) returns mathint = x+y;
        
        /**
         * My amazing invariant (should be @title)
         * @notice it is very amazing. Please notice
         */
        invariant myInvariant() addBv(x(),1) <= max_uint256 {
            preserved setX(uint y) with (env e) {
                require y < 1000;
            }
        }
    "};

    let parsed = Builder::new(src).build().unwrap();

    assert_eq!(parsed.len(), 3);

    assert_matches!(&parsed[0].ast, Ast::Methods { .. });
    assert_matches!(&parsed[1].ast, Ast::Definition { definition, ..} if definition == "x+y");
    assert_matches!(&parsed[2].ast, Ast::Invariant { .. });
}
