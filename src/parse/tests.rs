use crate::util::span_to_range::RangeConverter;
use crate::{AssociatedElement, NatSpec, Param, Tag};
use assert_matches::assert_matches;
use indoc::indoc;
use itertools::Itertools;
use lsp_types::{Position, Range};
use ropey::Rope;
use std::iter::zip;

fn parse_src(src: &str) -> Vec<NatSpec> {
    let rope = Rope::from_str(src);
    NatSpec::from_rope(rope)
}

macro_rules! param {
    ($ty: expr) => {
        ($ty.to_string(), None)
    };
    ($ty:expr, $name:expr) => {
        ($ty.to_string(), Some($name.to_string()))
    };
}

fn compare_params(expected_params: &[Param], actual_params: &[Param]) {
    assert_eq!(expected_params.len(), actual_params.len());

    for (expected, actual) in zip(expected_params, actual_params) {
        assert_eq!(expected.0, actual.0, "parsed param type is different");
        assert_eq!(expected.1, actual.1, "parsed param name is different");
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

    let parsed = parse_src(src);

    assert_eq!(parsed.len(), 5);

    assert_eq!(
        parsed[0],
        NatSpec::FreeForm {
            text: "# Section example".to_string(),
            range: range_from((0, 0), (1, 0))
        }
    );
    assert_eq!(
        parsed[1],
        NatSpec::FreeForm {
            text: "# Centered example".to_string(),
            range: range_from((2, 0), (3, 0))
        }
    );

    assert_eq!(
        parsed[2],
        NatSpec::FreeForm {
            text: "# Thick centered example".to_string(),
            range: range_from((4, 0), (7, 0))
        }
    );
    assert_eq!(
        parsed[3],
        NatSpec::FreeForm {
            text: "# Thick example".to_string(),
            range: range_from((8, 0), (11, 0))
        }
    );
    assert_eq!(
        parsed[4],
        NatSpec::FreeForm {
            text: "# Multiline example\nAdditional detail\nand more info".to_string(),
            range: range_from((12, 0), (17, 0))
        }
    );
}

fn range_from((s_line, s_character): (u32, u32), (e_line, e_character): (u32, u32)) -> Range {
    let start = Position::new(s_line, s_character);
    let end = Position::new(e_line, e_character);
    Range::new(start, end)
}

#[test]
fn doc_tag_spans_match_source() {
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

    let rope = Rope::from_str(src);
    let converter = RangeConverter::new(rope.clone());
    let natspecs: Vec<_> = NatSpec::from_rope(rope);

    let tags = natspecs.first().and_then(NatSpec::tags).unwrap();
    let tag_kinds: Vec<_> = tags.iter().map(|doc_tag| doc_tag.kind.clone()).collect();
    assert_eq!(
        tag_kinds,
        vec![
            Tag::Notice,
            Tag::Title,
            Tag::Unexpected(String::from("author")),
            Tag::Notice,
            Tag::Dev
        ]
    );

    let actual_tags: Vec<_> = tags
        .iter()
        .filter_map(|doc_tag| {
            let span = doc_tag.range.map(|range| converter.to_span(range))?;
            let actual_tag_from_src = &src[span];
            Some(actual_tag_from_src)
        })
        .collect();
    assert_eq!(actual_tags, vec!["@title", "@author", "@notice", "@dev"])
}

#[test]
fn doc_description_with_empty_line() {
    let src = indoc! {"
            /**
             * some stuff goes here
             * more stuff goes there
             *
             * last line was empty
             * and should have been ignored
             * @title A house for dogs
             * @notice Not for cats
             */
            function dogHouse() { {}
                string dog;
            }
        "};

    let parsed = parse_src(src);
    let first_tag = parsed
        .first()
        .and_then(NatSpec::tags)
        .and_then(<[_]>::first)
        .unwrap();

    assert_eq!(first_tag.kind, Tag::Notice);
    assert_eq!(first_tag.description, "some stuff goes here\nmore stuff goes there\nlast line was empty\nand should have been ignored");
}

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
            invariant goodMath(
                                uint a, 
                                int b,
                                string c
                               ) 
            {
            }
        "};

    let parsed = parse_src(src);
    let associated = parsed
        .first()
        .and_then(NatSpec::associated_element)
        .unwrap();

    assert_eq!(associated.name().unwrap(), "goodMath");

    let expected_params = [
        param!("uint", "a"),
        param!("int", "b"),
        param!("string", "c"),
    ];
    compare_params(&expected_params, associated.params().unwrap());
}

#[test]
fn comments_in_associated_element() {
    let src = indoc! {"
            /// this test checks
            /// that even if you put comments
            /// in problematic areas,
            /// parsing still works
            /**/
            //lorem ipsum dolor sit amet
            rule 
            ///// asdfasdfasdfasd
            ofLaw(string //randomtext
                       lapd
                       /**
                        * more random text
                        */
                        , string csny
                               ) { }
        "};

    let parsed = parse_src(src);
    let associated = parsed
        .first()
        .and_then(NatSpec::associated_element)
        .unwrap();

    assert_matches!(associated, AssociatedElement::Rule { .. });
    assert_eq!(associated.name().unwrap(), "ofLaw");

    let expected_params = [param!("string", "lapd"), param!("string", "csny")];
    compare_params(&expected_params, associated.params().unwrap());
}

#[test]
fn commented_out_blocks_are_ignored() {
    let src = indoc! {r#"
            /*
            /// This should not be parsed as a NatSpec doc,
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
             * note that this is valid starred natspec 
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

    let parsed = parse_src(src);
    assert!(
        parsed.is_empty(),
        "valid NatSpec blocks were parsed from commented out blocks"
    );
}

#[test]
fn commented_out_doc_followed_by_non_commented() {
    let src = indoc! {r#"
            /// @title the rule associated
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

    let parsed = parse_src(src);

    assert_eq!(parsed.len(), 2);
    assert!(parsed.iter().all(NatSpec::is_documentation));

    assert!(parsed[0].associated_element().is_none());
    assert_eq!(
        parsed[1]
            .associated_element()
            .and_then(AssociatedElement::name)
            .unwrap(),
        "bar"
    );
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

    let parsed = parse_src(src);

    let block = parsed[0]
        .associated_element()
        .and_then(AssociatedElement::block)
        .expect("could not capture code block");

    assert!(block.ends_with("{}{}{}{{}}{{{{   }}}}"));
}

#[test]
fn invariants() {
    let src = indoc! {r#"
    /**
     @title Valid Operator
     @notice Zero cannot be an operator.
    */
    invariant validOperator(address operator)
            beneficiaryOf(operator) != 0  <=>  ( operator != 0 && ownerOf(operator) != 0 && authorizerOf(operator) != 0 )

    /**
         @title Valid state of an operator ❌.
        @notice Operators with assets must have an owner, a beneficiary, and an authorizer.

            (unbondedValue(o) + lockedBonds(o)) > 0 ⟹
                ( ownerOf(o) ≠ 0 ⋀ beneficiaryOf(o) ≠ 0 ⋀ authorizerOf(o) ≠ 0 )
    */
    definition MAX_UINT160() returns uint256 = 1461501637330902918203684832716283019655932542975
    ;
        "#};

    let parsed = parse_src(src);
    assert_eq!(parsed.len(), 2);

    assert_matches!(
        parsed[0].associated_element().unwrap(),
        AssociatedElement::Invariant { .. }
    );
    assert_matches!(
        parsed[1].associated_element().unwrap(),
        AssociatedElement::Definition { .. }
    );
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

    let natspec = parse_src(src)
        .into_iter()
        .at_most_one()
        .expect("parses to exactly one element");

    assert_matches!(
        natspec.as_ref().and_then(NatSpec::associated_element),
        Some(AssociatedElement::Rule { .. })
    );
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

    let natspec = parse_src(src)
        .into_iter()
        .at_most_one()
        .expect("parses to exactly one element");

    if let Some(NatSpec::FreeForm { text, .. }) = natspec {
        assert_eq!(text, "## Verification of ERC1155Burnable\n\n`ERC1155Burnable` extends the `ERC1155` functionality by wrapping the internal\nmethods `_burn` and `_burnBatch` in the public methods `burn` and `burnBatch`,\nadding a requirement that the caller of either method be the account holding\nthe tokens or approved to act on that account's behalf.\n\n### Assumptions and Simplifications\n\n- No changes made using the harness\n\n### Properties");
    } else {
        panic!("should have been parsed as documentation")
    }
}
