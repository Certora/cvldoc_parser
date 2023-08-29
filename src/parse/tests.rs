mod cvl2;

use std::iter::Iterator;

use super::builder::Builder;
use super::Token;
use crate::CvlElement;
use crate::{util::SingleElement, Ast, Param, TagKind};
use assert_matches::assert_matches;
use color_eyre::eyre::{bail, Context};
use color_eyre::Report;
use indoc::indoc;

fn parse_exactly_one(src: &str) -> Result<CvlElement, Report> {
    let mut parsed = Builder::new(src).build().wrap_err("parsing failed")?;

    match parsed.len() {
        1 => Ok(parsed.remove(0)),
        n => bail!("expected single element, but found {n}"),
    }
}

fn parse_zero(src: &str) -> Result<(), Report> {
    let parsed = Builder::new(src).build().wrap_err("parsing failed")?;

    match parsed.len() {
        0 => Ok(()),
        n => bail!("expected zero elements, but found {n}"),
    }
}

#[test]
fn free_form_comments() {
    let src = indoc! {"
        /**** # Section example *************************/

        /*************** # Centered example **************/

        /********************************/
        /*** # Thick centered example */
        /********************************/

        /////////////////////////////////////////
        //// # Thick example                   ////
        /////////////////////////////////////////

        /***
         * # Multiline example
         * Additional detail
         * and more info
         */
    "};

    let parsed = Builder::new(src).build().unwrap();

    assert_eq!(parsed.len(), 5);

    assert_eq!(
        parsed[0].ast,
        Ast::FreeFormComment {
            text: "# Section example".to_string()
        }
    );
    assert_eq!(
        parsed[1].ast,
        Ast::FreeFormComment {
            text: "# Centered example".to_string()
        },
    );
    assert_eq!(
        parsed[2].ast,
        Ast::FreeFormComment {
            text: "# Thick centered example".to_string()
        },
    );
    assert_eq!(
        parsed[3].ast,
        Ast::FreeFormComment {
            text: "# Thick example".to_string()
        },
    );
    assert_eq!(
        parsed[4].ast,
        Ast::FreeFormComment {
            text: "# Multiline example\nAdditional detail\nand more info".to_string()
        },
    );
}

// fn range_from((s_line, s_character): (u32, u32), (e_line, e_character): (u32, u32)) -> Range {
//     let start = Position::new(s_line, s_character);
//     let end = Position::new(e_line, e_character);
//     Range::new(start, end)
// }

#[test]
fn doc_tag_kinds() {
    let src = indoc! {"
        /// hello hello hello
        /// world world world
        /// @title A simulator for trees
        /// and for everything green
        /// @author Larry A. Gardner
        /// @notice You can use this contract for only the most basic simulation
        /// @dev All function calls are currently implemented without side effects
        rule trees { }
    "};

    let parsed = Builder::new(src).build().unwrap();

    let tag_kinds = parsed[0]
        .doc
        .iter()
        .flatten()
        .cloned()
        .map(|doc_tag| doc_tag.kind);
    let expected = [
        TagKind::Notice,
        TagKind::Title,
        TagKind::Unexpected(String::from("author")),
        TagKind::Notice,
        TagKind::Dev,
    ];
    assert!(expected.into_iter().eq(tag_kinds));
}

// #[test]
// #[ignore = "requirements changed: now if a trimmed line is empty, we keep it"]
// fn doc_description_with_empty_line() {
//     let src = indoc! {"
//             /**
//              * some stuff goes here
//              * more stuff goes there
//              *
//              * last line was empty
//              * and should have been ignored
//              * @title A house for dogs
//              * @notice Not for cats
//              */
//             function dogHouse() { {}
//                 string dog;
//             }
//         "};
//     let parsed = parse_src(src);
//     let tags = data_of_first(&parsed).and_then(DocData::tags).unwrap();

//     assert_eq!(tags[0].kind, Tag::Notice);
//     assert_eq!(tags[0].description, "some stuff goes here\nmore stuff goes there\nlast line was empty\nand should have been ignored");
// }

#[test]
fn parsing_params() {
    let src = indoc! {"
        /**
         * this is here to check that params under documentation
         * are parsed correctly
         * @formula 1 + 1
         * @param a some number
         * @param b some other number
         * @param c not a number
         * @notice why are you still reading this
         */
        function goodMath(
                            uint a,
                            int b,
                            string c
                            )
        {
        }
    "};

    let parsed = parse_exactly_one(src).unwrap();
    let Ast::Function { name, params, .. } = parsed.ast else {
        panic!()
    };
    assert_eq!(name, "goodMath");

    let expected_params = [
        Param::new("uint", "a"),
        Param::new("int", "b"),
        Param::new("string", "c"),
    ];
    let params_are_equal = Iterator::eq(expected_params.iter(), params.iter());
    assert!(params_are_equal);
}

#[test]
fn comments_in_element() {
    let src = indoc! {"
        /// this test checks
        /// that even if you put comments
        /// in problematic areas,
        /// parsing still works
        /**/
        //lorem ipsum dolor sit amet
        rule
        // asdfasdfasdfasd
        ofLaw(string //randomtext
                    lapd
                    /*
                    * more random text
                    */
                    , string csny
                            ) { }
    "};

    let parsed = parse_exactly_one(src).unwrap();
    let Ast::Rule { name, params, .. } = parsed.ast else {
        panic!()
    };

    assert_eq!(name, "ofLaw");

    let expected_params = [Param::new("string", "lapd"), Param::new("string", "csny")];
    let params_are_equal = Iterator::eq(expected_params.iter(), params.iter());
    assert!(params_are_equal);
}

#[test]
fn commented_out_blocks_are_ignored() {
    let src = indoc! {r#"
        /*
        /// This should not be parsed as CVLDoc documentation,
        /// since the entire block is commented out.
        rule sanity {
            method f; env e; calldataarg args;
            f(e, args);
            assert false,
                "This rule should always fail";
        }
        */
        /*
        /**
         * this one should not be parsed either.
         * note that this is valid starred cvldoc
         * doc, and as such it ends with the
         * same terminator that ends a regular CVL comment
         * which could cause parsing ambiguities.
         */
        rule insanity {
            method f; env e; calldataarg args;
            f(e, args);
            assert true,
                "This rule should always pass";
        }
        */
    "#};

    let lexed = Builder::new(src).lex().unwrap();

    assert!(lexed
        .into_iter()
        .all(|(tok, _span)| matches!(tok, Token::SingleLineComment | Token::MultiLineComment)),);
}

#[test]
fn commented_out_doc_followed_by_non_commented() {
    let src = indoc! {r#"
        /// @notice the rule associated
        /// with this doc is commented out,
        /// so it should be considered as a
        /// documentation with no associated element
        // rule foo(method f) {
        //     env e; calldataarg args;
        //     // require inRecoveryMode(e); // alternative way, should try both
        /* */ //

        //     assert !lastReverted, "recovery mode must not fail";
        // }

        ///@title this is another rule with a doc
        /// this one's associated element is not ,
        /// commented out and so SHOULD be considered as
        /// having an associated element.
        /// furthermore, it should be parsed as a separate documentation
        /// block from the one above.
        rule bar(method f) {
            thank_you_for_playing_wing_commander();
        }
    "#};

    let cvl_element = parse_exactly_one(src).unwrap();
    let element_doc = cvl_element.doc.unwrap().single_element();

    assert_eq!(element_doc.kind, TagKind::Title);
    assert_eq!(cvl_element.ast.name(), Some("bar"));
}

#[test]
fn grabbing_blocks() {
    let src = indoc! {r#"
            /**
             * this checks that nested blocks are grabbed
             */
            function of(Christmas past) {
                if (true) {
                    do_this();
                } else {
                    do_that();
                }
                {}{}{}{{}}{{{{   }}}}
            }fizz buzz{}
        "#};

    let ast = parse_exactly_one(src).unwrap().ast;

    assert!(ast.block().unwrap().ends_with("{}{}{}{{}}{{{{   }}}}"));
}

#[test]
fn invariants() {
    let src = indoc! {r#"
        /**
         @title Valid Operator
        @notice Zero cannot be an operator.
        */
        invariant validOperator(address operator)
                beneficiaryOf(operator) != 0  <=>  ( operator != 0 && ownerOf(operator) != 0 && authorizerOf(operator) != 0 );

        /**
             @title Valid state of an operator ❌.
            @notice Operators with assets must have an owner, a beneficiary, and an authorizer.

                (unbondedValue(o) + lockedBonds(o)) > 0 ⟹
                    ( ownerOf(o) ≠ 0 ⋀ beneficiaryOf(o) ≠ 0 ⋀ authorizerOf(o) ≠ 0 )
        */
        definition MAX_UINT160() returns uint256 = 1461501637330902918203684832716283019655932542975
        ;
    "#};

    let parsed = Builder::new(src).build().unwrap();
    assert_eq!(parsed.len(), 2);

    assert_matches!(parsed[0].ast, Ast::Invariant { .. });
    assert_matches!(parsed[1].ast, Ast::Definition { .. });
}

#[test]
fn rules_without_parameters() {
    let src = indoc! {r#"
        /// Burning a larger amount of a token must reduce that token's balance more
        /// than burning a smaller amount.
        /// n.b. This rule holds for `burnBatch` as well due to rules establishing
        /// appropriate equivance between `burn` and `burnBatch` methods.
        rule burnAmountProportionalToBalanceReduction {
            storage beforeBurn = lastStorage;
            env e;

            address holder; uint256 token;
            mathint startingBalance = balanceOf(holder, token);
            uint256 smallBurn; uint256 largeBurn;
            require smallBurn < largeBurn;

            // smaller burn amount
            burn(e, holder, token, smallBurn) at beforeBurn;
            mathint smallBurnBalanceChange = startingBalance - balanceOf(holder, token);

            // larger burn amount
            burn(e, holder, token, largeBurn) at beforeBurn;
            mathint largeBurnBalanceChange = startingBalance - balanceOf(holder, token);

            assert smallBurnBalanceChange < largeBurnBalanceChange,
                "A larger burn must lead to a larger decrease in balance";
        }
    "#};

    let element = parse_exactly_one(src).unwrap();

    assert!(!element.doc.unwrap().is_empty());
    assert_matches!(element.ast, Ast::Rule { .. });
}

#[test]
fn multiline_slashed_freeform_concatenates_to_a_single_comment() {
    let src = indoc! {r#"
        //// ## Verification of ERC1155Burnable
        ////
        //// `ERC1155Burnable` extends the `ERC1155` functionality by wrapping the internal
        //// methods `_burn` and `_burnBatch` in the public methods `burn` and `burnBatch`,
        //// adding a requirement that the caller of either method be the account holding
        //// the tokens or approved to act on that account's behalf.
        ////
        //// ### Assumptions and Simplifications
        ////
        //// - No changes made using the harness
        ////
        //// ### Properties

        methods {
            balanceOf(address, uint256) returns uint256 envfree
            isApprovedForAll(address,address) returns bool envfree
        }
    "#};

    let parsed = Builder::new(src).build().unwrap();
    let element = &parsed[0];

    let expected = "## Verification of ERC1155Burnable\n\n`ERC1155Burnable` extends the `ERC1155` functionality by wrapping the internal\nmethods `_burn` and `_burnBatch` in the public methods `burn` and `burnBatch`,\nadding a requirement that the caller of either method be the account holding\nthe tokens or approved to act on that account's behalf.\n\n### Assumptions and Simplifications\n\n- No changes made using the harness\n\n### Properties";
    let Ast::FreeFormComment { text } = &element.ast else {
        panic!("should have been parsed as documentation");
    };
    assert_eq!(text, expected);
}

#[test]
fn crlf() {
    let src_with_crlf_encoding = indoc! {r#"
        /***
         * # testing the natspec parser.
         * This is made by Gabriel  it contains all
         * known tags
         ***/
        methods {
            get5() returns uint envfree
            init_state() envfree
            setX(uint256) envfree
            getX() returns uint envfree
            getXCanRevert(uint256) returns uint envfree
            twoReturns(uint256) returns (uint256,uint256) envfree
            threeReturns(uint256,uint256) returns (uint256,uint256,uint256)
        }
    "#}
    .replace('\n', "\r\n");

    let parsed = Builder::new(&src_with_crlf_encoding).build().unwrap();

    let Ast::FreeFormComment { text } = &parsed[0].ast else {
        panic!()
    };

    assert_eq!(
        text,
        "# testing the natspec parser.\r\nThis is made by Gabriel  it contains all\r\nknown tags"
    );
}

#[test]
fn methods_with_whitespace_between_name_and_params() {
    let src = indoc! {r#"
        /// When a contract is in a paused state, transfer methods must revert.
        rule transferMethodsRevertWhenPaused (method f)
        filtered {
            f -> f.selector == safeTransferFrom(address,address,uint256,uint256,bytes).selector
            || f.selector == safeBatchTransferFrom(address,address,uint256[],uint256[],bytes).selector
        }
        {
            require paused();

            env e; calldataarg args;
            f@withrevert(e, args);

            assert lastReverted,
                "Transfer methods must revert in a paused contract";
        }
    "#};

    assert_matches!(parse_exactly_one(src).unwrap().ast, Ast::Rule { .. });
}

#[test]
fn freeform_stars_without_text() {
    let src = indoc! { r#"
        /******************************************************************************/
        ghost mapping(uint256 => mathint) sumOfBalances {
            init_state axiom forall uint256 token . sumOfBalances[token] == 0;
        }
    "#};

    let parsed = Builder::new(src).build().unwrap();

    let Ast::FreeFormComment { text } = &parsed[0].ast else {
        panic!()
    };
    assert!(text.is_empty());
}

#[test]
fn freeform_stars_before_and_after() {
    let src = indoc! { r#"
        /******************************************************************************/
        /// The sum of the balances over all users must equal the total supply for a
        /// given token.
        invariant total_supply_is_sum_of_balances(uint256 token)
            sumOfBalances[token] == totalSupply(token)
            {
                preserved {
                    requireInvariant balanceOfZeroAddressIsZero(token);
                }
            }

        /******************************************************************************/
    "#};

    let parsed = Builder::new(src).build().unwrap();
    let expected_name = "total_supply_is_sum_of_balances";

    let needle = parsed
        .iter()
        .find(|element| element.ast.name() == Some(expected_name));
    assert!(needle.is_some());
}

#[test]
fn span_contains_both_doc_and_associated_element() {
    let src = indoc! { r#"
        /// If a method call reduces account balances, the caller must be either the
        /// holder of the account or approved to act on the holder's behalf.
        rule onlyHolderOrApprovedCanReduceBalance(method f)
        {
            address holder; uint256 token; uint256 amount;
            uint256 balanceBefore = balanceOf(holder, token);

            env e; calldataarg args;
            f(e, args);

            uint256 balanceAfter = balanceOf(holder, token);

            assert balanceAfter < balanceBefore => e.msg.sender == holder || isApprovedForAll(holder, e.msg.sender),
                "An account balance may only be reduced by the holder or a holder-approved agent";
        }
    "#};

    assert_eq!(parse_exactly_one(src).unwrap().raw(), src.trim());
}

#[test]
fn raw_capture_for_multi_line_doc() {
    let src = indoc! { r#"
        /**
         * @title takeTwoEnvs function
         * @param e1 - first environment
         * @param e2 - second environment
         **/
        function takeTwoEnvs(env e1, env e2) {
            require e1.msg.value == 0;
            require e1.msg.sender == e2.msg.sender;
        }
    "#};

    assert!(parse_exactly_one(src).unwrap().raw().starts_with("/**"));
}

#[test]
fn blocks_where_brackets_are_not_separated_by_whitespace() {
    let src = indoc! {"
        /**
         * If deadline increases then we are in `deadlineExtended` state and `castVote`
         * was called.
         * @dev RULE PASSING
         * @dev ADVANCED SANITY PASSING 
         */ 
        rule deadlineChangeEffects(method f) filtered {f -> !f.isView} {
            env e; calldataarg args; uint256 pId;
        
            requireInvariant quorumReachedEffect(e, pId);
            
            uint256 deadlineBefore = proposalDeadline(pId);
            f(e, args);
            uint256 deadlineAfter = proposalDeadline(pId);
            
            assert(deadlineAfter > deadlineBefore => latestCastVoteCall() == e.block.number && deadlineExtended(e, pId));
        }
    "};

    let element = parse_exactly_one(src).unwrap();
    let Ast::Rule { filters, .. } = &element.ast else {
        panic!()
    };

    assert_eq!(filters.as_ref().unwrap(), "{f -> !f.isView}");
}

#[test]
fn variable_char_lengths() {
    let src = indoc! { r#"
        /***
        🔥🔥🔥💯 frfr
        */
        methods {
            𝇇_𝇇
        }

        //////////////
        //// Text ////
        //////////////
    "#};

    let parsed = Builder::new(src).build().unwrap();
    assert_eq!(parsed.len(), 3);

    let Ast::FreeFormComment { text } = &parsed[0].ast else {
        panic!();
    };
    assert_eq!(text, "🔥🔥🔥💯 frfr");

    let Ast::FreeFormComment { text } = &parsed[2].ast else {
        panic!();
    };
    assert_eq!(text, "Text");

    assert_eq!(parsed[0].raw(), "/***\n🔥🔥🔥💯 frfr\n*/");
    assert_eq!(parsed[1].raw(), "methods {\n    𝇇_𝇇\n}");
    assert_eq!(
        parsed[2].raw(),
        "//////////////\n//// Text ////\n//////////////\n"
    );
}

#[test]
fn repeated_iterations_caused_by_improper_recovery() {
    let src = "foo { function }";

    let _ = Builder::new(src).build();
}
