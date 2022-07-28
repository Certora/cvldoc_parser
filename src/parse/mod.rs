pub mod builder;
mod helpers;

use self::builder::UnderDoc;
use crate::util::span_to_range::Spanned;
use builder::NatSpecBuilder;
use chumsky::prelude::*;
use helpers::*;

fn free_form_comment<'src>() -> BoxedParser<'src, char, NatSpecBuilder, Simple<char>> {
    let padding_before_header = horizontal_ws()
        .then(just('#'))
        .then(horizontal_ws())
        .boxed();

    let slashes = just('/').repeated().at_least(4);
    let thick_slashed_padding = slashes.then(newline_or_end());

    let slashed_header = slashes
        .ignore_then(padding_before_header.clone())
        .ignore_then(take_to_newline_or_end())
        .collect()
        .map(|line: String| {
            let padding = &[' ', '\t', '/'];
            line.trim_end_matches(padding).to_string()
        })
        .boxed();

    let slashed_single_line_free_form = slashed_header.clone();
    let slashed_thick_free_form = slashed_header.padded_by(thick_slashed_padding);

    let stars = just('*').repeated().at_least(3);
    let thick_starred_padding = just('/').then(stars).then(just('/')).then(newline_or_end());

    let starred_header = just('/')
        .ignore_then(stars)
        .ignore_then(padding_before_header)
        .ignore_then(take_to_starred_terminator())
        .then_ignore(newline_or_end())
        .collect()
        .map(|line: String| {
            let padding = &[' ', '\t', '*'];
            line.trim_end_matches(padding).to_string()
        })
        .boxed();

    let starred_single_line_free_form = starred_header.clone();
    let starred_thick_free_form = starred_header.padded_by(thick_starred_padding).boxed();

    let free_form_single_line = choice((
        slashed_single_line_free_form,
        slashed_thick_free_form,
        starred_single_line_free_form,
        starred_thick_free_form,
    ))
    .map(|header| NatSpecBuilder::FreeFormComment {
        header,
        block: None,
    });

    let multi_line_first_line = just('/').then(stars).then(just('\n')).boxed();
    let multi_line_body = take_until(just('#'))
        .ignore_then(take_to_starred_terminator())
        .then_ignore(newline_or_end())
        .collect()
        .boxed();
    let free_form_multi_line = multi_line_first_line
        .ignore_then(multi_line_body)
        .map(NatSpecBuilder::free_form_multi_line_from_body)
        .boxed();

    choice((free_form_single_line, free_form_multi_line)).boxed()
}

fn optional_token_separator<'src>() -> BoxedParser<'src, char, (), Simple<char>> {
    let single_line_comment = just("//").then(take_to_newline_or_end());
    let multi_line_comment = just("/*").then(take_to_starred_terminator());
    let comments = choice((single_line_comment, multi_line_comment))
        .padded()
        .repeated()
        .padded();

    comments.ignored().boxed()
}

fn mandatory_token_separator<'src>() -> BoxedParser<'src, char, (), Simple<char>> {
    let mandatory_ws = text::whitespace().at_least(1);

    mandatory_ws.ignore_then(optional_token_separator()).boxed()
}

fn under_doc<'src>() -> BoxedParser<'src, char, UnderDoc, Simple<char>> {
    let decl_under_natspec = {
        let decl_kind = text::ident().try_map(|kw: String, span| {
            let kind = kw.as_str().try_into();
            kind.map_err(|e| Simple::custom(span, e))
        });
        let decl_name = text::ident();

        let decl = decl_kind
            .then_ignore(mandatory_token_separator())
            .then(decl_name);

        decl.padded_by(optional_token_separator()).boxed()
    };

    let params_under_natspec = {
        let args = text::ident()
            .then_ignore(mandatory_token_separator())
            .then(text::ident())
            .padded_by(optional_token_separator())
            .boxed();

        args.separated_by(just(','))
            .delimited_by(just('('), just(')'))
            .boxed()
    };

    decl_under_natspec
        .then(params_under_natspec.or_not().map(Option::unwrap_or_default))
        .then_ignore(optional_token_separator())
        .then_ignore(just('{').rewind())
        .map(|((kind, name), params)| UnderDoc(kind, name, params))
        .boxed()
}

fn natspec_doc<'src>() -> BoxedParser<'src, char, NatSpecBuilder, Simple<char>> {
    let spanned_slashed_line = just("///")
        .ignore_then(none_of('/').rewind())
        .ignore_then(horizontal_ws())
        .ignore_then(
            take_to_newline_or_end()
                .collect()
                .map_with_span(|trimmed_line, span| (trimmed_line, span)),
        )
        .boxed();

    let slashed_documentation = spanned_slashed_line.repeated().at_least(1).boxed();

    let starred_documentation = just("/**")
        .then(none_of("*/").rewind())
        .ignore_then(take_to_starred_terminator().map_with_span(builder::split_starred_doc_lines))
        .boxed();

    choice([slashed_documentation, starred_documentation])
        .then(under_doc().or_not())
        .map(
            |(spanned_body, element_under_doc)| NatSpecBuilder::Documentation {
                spanned_body,
                element_under_doc,
            },
        )
        .boxed()
}

pub(super) fn parser() -> impl Parser<char, Vec<Spanned<NatSpecBuilder>>, Error = Simple<char>> {
    let natspec = choice([free_form_comment(), natspec_doc()]);

    natspec
        .recover_with(skip_until(['\n', ' '], |_| NatSpecBuilder::ParseError).consume_end())
        .map_with_span(|builder, span| (builder, span))
        .repeated()
        .boxed()
}

#[cfg(test)]
mod tests {
    use crate::util::span_to_range::RangeConverter;
    use crate::{DeclarationKind, NatSpec, Tag};
    use indoc::indoc;
    use ropey::Rope;
    use std::iter::zip;

    fn parse_src(src: &str) -> Vec<NatSpec> {
        let rope = Rope::from_str(src);
        let (natspecs, _ranges): (Vec<_>, Vec<_>) = NatSpec::from_rope(rope).into_iter().unzip();

        natspecs
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
                    header: "Section example".to_string()
                },
                NatSpec::SingleLineFreeForm {
                    header: "Centered example".to_string()
                },
                NatSpec::SingleLineFreeForm {
                    header: "Thick centered example".to_string()
                },
                NatSpec::SingleLineFreeForm {
                    header: "Thick example".to_string()
                },
                NatSpec::MultiLineFreeForm {
                    header: "Multiline example".to_string(),
                    block: "Additional detail\nand more info".to_string()
                },
            ]
        )
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
        let (natspecs, _ranges): (Vec<_>, Vec<_>) = NatSpec::from_rope(rope).into_iter().unzip();

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
            function dogHouse() {
                string dog;
            }
        "};

        let parsed = parse_src(src);
        let first_tag = parsed
            .first()
            .and_then(NatSpec::tags)
            .and_then(|tags| tags.first())
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

        assert_eq!(associated.name, "goodMath");

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
            // asdfasdfasdfasd
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
        assert_eq!(associated.name, "ofLaw");

        let expected_params = [("string", "lapd"), ("string", "csny")];
        compare_params(&expected_params, &associated.params);
    }
}
