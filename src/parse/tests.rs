use crate::parse::lexed::{Ast, Token};
use crate::util::span_to_range::RangeConverter;
use crate::{AssociatedElement, CvlDoc, DocData, Param, Tag};
use assert_matches::assert_matches;
use chumsky::{Parser, Stream};
use color_eyre::eyre::bail;
use color_eyre::Report;
use color_eyre::Result;
use indoc::indoc;
use itertools::Itertools;
use lsp_types::{Position, Range};
use ropey::Rope;
use std::iter::zip;
use std::ops::Not;
use std::path::Path;

use super::builder::CvlDocBuilder;
use super::lexed;

fn parse_src(src: &str) -> Vec<CvlDoc> {
    let rope = Rope::from_str(src);
    CvlDoc::from_rope(rope)
}

macro_rules! param {
    ($ty: expr) => {
        ($ty.to_string(), None)
    };
    ($ty:expr, $name:expr) => {
        ($ty.to_string(), Some($name.to_string()))
    };
}

trait PostfixDbg {
    fn dbg(self) -> Self;
}

impl<T: std::fmt::Debug> PostfixDbg for T {
    fn dbg(self) -> Self {
        dbg!(self)
    }
}

impl CvlDoc {
    fn data(self) -> DocData {
        self.data
    }
}

fn data_of_first(docs: &[CvlDoc]) -> Option<&DocData> {
    docs.first().map(|doc| &doc.data)
}

fn parse_to_exactly_one_element(src: &str) -> Result<CvlDoc, Report> {
    match parse_src(src).into_iter().at_most_one() {
        Ok(Some(doc)) => Ok(doc),
        _ => bail!("should parse to exactly one element"),
    }
}

fn compare_params(expected_params: &[Param], actual_params: &[Param]) {
    assert_eq!(expected_params.len(), actual_params.len());

    for (expected, actual) in zip(expected_params, actual_params) {
        assert_eq!(expected.0, actual.0, "parsed param type is different");
        assert_eq!(expected.1, actual.1, "parsed param name is different");
    }
}

fn find_by_name_of_associated_element<'a>(
    expected_name: &str,
    parsed_docs: &'a [CvlDoc],
) -> Option<&'a CvlDoc> {
    parsed_docs.iter().find(|doc| {
        let DocData::Documentation { associated: Some(assoc), .. } = &doc.data else { return false; };
        let Some(name) = assoc.name() else { return false; };
        name == expected_name
    })
}

fn parse_from_path(path: impl AsRef<Path>) -> Result<Vec<CvlDoc>> {
    let spec = std::fs::read_to_string(path)?;
    Ok(parse_src(spec.as_str()))
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

    assert_eq!(parsed[0].range, range_from((0, 0), (1, 0)));
    assert_eq!(
        parsed[0].data,
        DocData::FreeForm("# Section example".to_string())
    );
    assert_eq!(parsed[1].range, range_from((2, 0), (3, 0)));
    assert_eq!(
        parsed[1].data,
        DocData::FreeForm("# Centered example".to_string()),
    );
    assert_eq!(parsed[2].range, range_from((4, 0), (7, 0)));
    assert_eq!(
        parsed[2].data,
        DocData::FreeForm("# Thick centered example".to_string()),
    );
    assert_eq!(parsed[3].range, range_from((8, 0), (11, 0)));
    assert_eq!(
        parsed[3].data,
        DocData::FreeForm("# Thick example".to_string()),
    );
    assert_eq!(parsed[4].range, range_from((12, 0), (17, 0)));
    assert_eq!(
        parsed[4].data,
        DocData::FreeForm("# Multiline example\nAdditional detail\nand more info".to_string()),
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

    let parsed = parse_src(src);
    let tags = data_of_first(&parsed).and_then(DocData::tags).unwrap();
    let tag_kinds = tags
        .iter()
        .map(|doc_tag| doc_tag.kind.clone())
        .collect_vec();
    assert_eq!(
        tag_kinds,
        [
            Tag::Notice,
            Tag::Title,
            Tag::Unexpected(String::from("author")),
            Tag::Notice,
            Tag::Dev
        ]
    );

    let converter = RangeConverter::new(Rope::from_str(src));
    let actual_tags = tags
        .iter()
        .filter_map(|doc_tag| {
            let span = doc_tag.range.map(|range| converter.to_span(range))?;
            let actual_tag_from_src = &src[span];
            Some(actual_tag_from_src)
        })
        .collect_vec();
    assert_eq!(actual_tags, ["@title", "@author", "@notice", "@dev"])
}

#[test]
#[ignore = "requirements changed: now if a trimmed line is empty, we keep it"]
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
    let tags = data_of_first(&parsed).and_then(DocData::tags).unwrap();

    assert_eq!(tags[0].kind, Tag::Notice);
    assert_eq!(tags[0].description, "some stuff goes here\nmore stuff goes there\nlast line was empty\nand should have been ignored");
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
    let associated = data_of_first(&parsed)
        .and_then(DocData::associated_element)
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
    let associated = data_of_first(&parsed)
        .and_then(DocData::associated_element)
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

    let parsed = parse_src(src);
    assert!(
        parsed.is_empty(),
        "valid CVLDoc blocks were parsed from commented out blocks"
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
    assert!(parsed.iter().all(|doc| doc.data.is_documentation()));

    assert!(parsed[0].data.associated_element().is_none());
    assert_eq!(
        parsed[1]
            .data
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
    let block = data_of_first(&parsed)
        .unwrap()
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
        parsed[0].data.associated_element().unwrap(),
        AssociatedElement::Invariant { .. }
    );
    assert_matches!(
        parsed[1].data.associated_element().unwrap(),
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

    let parsed = parse_to_exactly_one_element(src).unwrap();

    assert_matches!(
        parsed.data.associated_element(),
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

    let parsed = parse_to_exactly_one_element(src).unwrap();

    let expected = "## Verification of ERC1155Burnable\n\n`ERC1155Burnable` extends the `ERC1155` functionality by wrapping the internal\nmethods `_burn` and `_burnBatch` in the public methods `burn` and `burnBatch`,\nadding a requirement that the caller of either method be the account holding\nthe tokens or approved to act on that account's behalf.\n\n### Assumptions and Simplifications\n\n- No changes made using the harness\n\n### Properties";
    match parsed.data {
        DocData::FreeForm(text) => assert_eq!(text, expected),
        _ => panic!("should have been parsed as documentation"),
    }
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

    let parsed = parse_to_exactly_one_element(&src_with_crlf_encoding).unwrap();

    match parsed.data {
        DocData::FreeForm(text) => assert_eq!(text, "# testing the natspec parser.\r\nThis is made by Gabriel  it contains all\r\nknown tags"),
        _ => panic!("should have been parsed as documentation"),
    }
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

    let parsed = parse_to_exactly_one_element(src).unwrap();

    if let DocData::Documentation { associated, .. } = parsed.data {
        assert_matches!(associated, Some(AssociatedElement::Rule { .. }));
    } else {
        panic!("should have been parsed as documentation");
    }
}

#[test]
fn freeform_stars_without_text() {
    // let src = "definition harness_isListed(address a, uint i) returns bool = 0 <= i && i < shadowLenArray() && shadowArray(i) == a ;";
    let src = indoc! { r#"
    /******************************************************************************/
    ghost mapping(uint256 => mathint) sumOfBalances {
        init_state axiom forall uint256 token . sumOfBalances[token] == 0;
    }
    "#};

    let parsed_doc_data = parse_to_exactly_one_element(src).map(CvlDoc::data);

    let Ok(DocData::FreeForm(s)) = parsed_doc_data else { panic!() };
    assert!(s.is_empty());
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

    let expected_name = "total_supply_is_sum_of_balances";
    let parsed_docs = parse_src(src);

    assert!(find_by_name_of_associated_element(expected_name, &parsed_docs).is_some());
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

    let Ok(CvlDoc { raw, .. }) = parse_to_exactly_one_element(src) else { panic!() };
    assert_eq!(raw, src.trim());
}

#[test]
fn new_parser() {
    let src = indoc! { r#"
    //// ## Verification of ERC1155Supply
    //// 
    //// `ERC1155Supply` extends the `ERC1155` functionality. The contract creates a publicly callable `totalSupply` wrapper for the private `_totalSupply` method, a public `exists` method to check for a positive balance of a given token, and updates `_beforeTokenTransfer` to appropriately change the mapping `_totalSupply` in the context of minting and burning tokens.
    //// 
    //// ### Assumptions and Simplifications
    //// - The `exists` method was wrapped in the `exists_wrapper` method because `exists` is a keyword in CVL.
    //// - The public functions `burn`, `burnBatch`, `mint`, and `mintBatch` were implemented in the harnesssing contract make their respective internal functions callable by the CVL. This was used to test the increase and decrease of `totalSupply` when tokens are minted and burned. 
    //// - We created the `onlyOwner` modifier to be used in the above functions so that they are not called in unrelated rules.
    //// 
    //// ### Properties


    methods {
        totalSupply(uint256) returns uint256 envfree
        balanceOf(address, uint256) returns uint256 envfree
        exists_wrapper(uint256) returns bool envfree
        owner() returns address envfree
    }
    
    /// Given two different token ids, if totalSupply for one changes, then
    /// totalSupply for other must not.
    rule token_totalSupply_independence(method f)
    filtered {
        f -> f.selector != safeBatchTransferFrom(address,address,uint256[],uint256[],bytes).selector
    }
    {
        uint256 token1; uint256 token2;
        require token1 != token2;

        uint256 token1_before = totalSupply(token1);
        uint256 token2_before = totalSupply(token2);

        env e; calldataarg args;
        require e.msg.sender != owner(); // owner can call mintBatch and burnBatch in our harness
        f(e, args);

        uint256 token1_after = totalSupply(token1);
        uint256 token2_after = totalSupply(token2);

        assert token1_after != token1_before => token2_after == token2_before,
            "methods must not change the total supply of more than one token";
    }

    /******************************************************************************/

    ghost mapping(uint256 => mathint) sumOfBalances {
        init_state axiom forall uint256 token . sumOfBalances[token] == 0;
    }

    hook Sstore _balances[KEY uint256 token][KEY address user] uint256 newValue (uint256 oldValue) STORAGE {
        sumOfBalances[token] = sumOfBalances[token] + newValue - oldValue;
    }

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

    /// The balance of a token for the zero address must be zero.
    invariant balanceOfZeroAddressIsZero(uint256 token)
        balanceOf(0, token) == 0

    /// If a user has a token, then the token should exist.
    rule held_tokens_should_exist {
        address user; uint256 token;

        requireInvariant balanceOfZeroAddressIsZero(token);

        // This assumption is safe because of total_supply_is_sum_of_balances
        require balanceOf(user, token) <= totalSupply(token);

        // note: `exists_wrapper` just calls `exists`
        assert balanceOf(user, token) > 0 => exists_wrapper(token),
            "if a user's balance for a token is positive, the token must exist";
    }

    /******************************************************************************/
    /*
    rule sanity {
        method f; env e; calldataarg args;

        f(e, args);

        assert false;
    }
    */

    "#};
    let mut after_lexing = lexed::lexer().parse(src).unwrap();
    let len = src.chars().count();

    after_lexing.retain(|(tok, _span)| !matches!(tok, Token::SingleLineComment | Token::MultiLineComment));

    // after_lexing
    //     .iter()
    //     .map(|(tok, _)| tok)
    //     .for_each(|tok| println!("{tok:?}"));

    let mut stream = Stream::from_iter(len..len + 1, after_lexing.into_iter());
    // stream.fetch_tokens().for_each(|(token, span)| println!("{token:?}"));
    let converter = RangeConverter::new(Rope::from_str(src));

    let (after_parsing, errors) = lexed::actual_parser().parse_recovery(stream);
    let parsed = after_parsing.unwrap();

    for p in parsed
        .into_iter()
        .filter(|p| !matches!(p, &Ast::ParseError))
    {
        // let Ast::Documentation(..) = p else { continue };
        let processed = p.process(converter.clone(), src);
        dbg!(processed);
    }
    // let Ast::FunctionDecl { name, params, returns, block } = parsed[0].clone() else { panic!() };

    // dbg!(params);
}
