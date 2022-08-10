use crate::util::span_to_range::RangeConverter;
use crate::{DeclarationKind, NatSpec, Tag};
use indoc::indoc;
use lsp_types::{Position, Range};
use ropey::Rope;
use std::iter::zip;

fn parse_src(src: &str) -> Vec<NatSpec> {
    let rope = Rope::from_str(src);
    NatSpec::from_rope(rope)
}

fn compare_params(expected_params: &[(&str, &str)], actual_params: &[(String, String)]) {
    assert_eq!(
        expected_params.len(),
        actual_params.len(),
        "not all params were parsed"
    );

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

    assert_eq!(
        parsed,
        vec![
            NatSpec::SingleLineFreeForm {
                header: "Section example".to_string(),
                range: range_from((0, 0), (1, 0))
            },
            NatSpec::SingleLineFreeForm {
                header: "Centered example".to_string(),
                range: range_from((2, 0), (3, 0))
            },
            NatSpec::SingleLineFreeForm {
                header: "Thick centered example".to_string(),
                range: range_from((4, 0), (7, 0))
            },
            NatSpec::SingleLineFreeForm {
                header: "Thick example".to_string(),
                range: range_from((8, 0), (11, 0))
            },
            NatSpec::MultiLineFreeForm {
                header: "Multiline example".to_string(),
                block: "Additional detail\nand more info".to_string(),
                range: range_from((12, 0), (17, 0))
            },
        ]
    )
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

    assert_eq!(associated.name.clone().unwrap(), "goodMath");

    let expected_params = [("uint", "a"), ("int", "b"), ("string", "c")];
    compare_params(&expected_params, &associated.params);
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

    assert_eq!(associated.kind, DeclarationKind::Rule);
    assert_eq!(associated.name.clone().unwrap(), "ofLaw");

    let expected_params = [("string", "lapd"), ("string", "csny")];
    compare_params(&expected_params, &associated.params);
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
            .and_then(|associated| associated.name.as_ref())
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
        .and_then(|elem| elem.block.as_ref())
        .expect("could not capture code block");

    assert_eq!(
        block,
        indoc! { "{
        if (true) {
            do_this();
        } else {
            do_that();
        }
        {}{}{}{{}}{{{{   }}}}
    }"}
    );
}

// #[test]
// fn invariants() {
//     let src = indoc! {r#"
//         /**
//         * some stuff
//         * @title A house for dogs
//         * @param fizz this param does not exist
//         */
//         // an invariant to assume, proven separately
// definition delegate_and_count_inv(address u, uint8 type) returns bool =
// 	(u != 0 && getDelegatee(u, type) != 0 && getDelegatee(u, type) != u) => ((type == VOTING_POWER() => _votingSnapshotsCounts(u) > 0) && (type == PROPOSITION_POWER() => getPropositionPowerSnapshotCount(u) > 0));
//     "#};

//     let parsed = parse_src(src);
//     dbg!(parsed[0].associated_element().unwrap());
// }
