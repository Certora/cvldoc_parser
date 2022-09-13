mod associated_element;
pub mod builder;
mod helpers;

use self::associated_element::associated_element;
use crate::util::span_to_range::Spanned;
use builder::NatSpecBuilder;
use chumsky::prelude::*;
use helpers::*;
use itertools::Itertools;

fn free_form_comment<'src>() -> BoxedParser<'src, char, NatSpecBuilder, Simple<char>> {
    let slashes = just('/').repeated().at_least(4);
    let thick_slashed_padding = slashes.then(newline_or_end());

    let slashed_free_form_line = slashes
        .ignore_then(horizontal_ws())
        .ignore_then(take_to_newline_or_end())
        .collect()
        .map(|line: String| {
            let padding = &[' ', '\t', '/'];
            line.trim_end_matches(padding).to_string()
        })
        .boxed();

    let slashed_free_form = slashed_free_form_line
        .clone()
        .repeated()
        .at_least(1)
        .map(|body| body.into_iter().join("\n"))
        .boxed();
    let slashed_thick_free_form = slashed_free_form_line
        .padded_by(thick_slashed_padding)
        .boxed();

    let stars = just('*').repeated().at_least(3);
    let thick_starred_padding = just('/').then(stars).then(just('/')).then(newline_or_end());

    let starred_header = just('/')
        .ignore_then(stars)
        .ignore_then(horizontal_ws())
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

    let starred_multi_line_first_line = just("/***").then(newline()).boxed();
    let starred_body = take_to_starred_terminator()
        .then_ignore(newline_or_end())
        .collect()
        .map(|body: String| {
            let padding: &[_] = &[' ', '\t', '*', '\n'];
            body.trim_end()
                .lines()
                .map(|line| line.trim_matches(padding))
                .join("\n")
        })
        .boxed();
    let starred_free_form = starred_multi_line_first_line
        .ignore_then(starred_body)
        .boxed();

    choice([
        slashed_thick_free_form,
        slashed_free_form,
        starred_free_form,
        starred_thick_free_form,
        starred_single_line_free_form,
    ])
    .map_with_span(|text, span| NatSpecBuilder::FreeFormComment { text, span })
    .boxed()
}

fn commented_out_block<'src>() -> BoxedParser<'src, char, NatSpecBuilder, Simple<char>> {
    multi_line_cvl_comment()
        .to(NatSpecBuilder::CommentedOutBlock)
        .boxed()
}

fn commented_out_line<'src>() -> BoxedParser<'src, char, NatSpecBuilder, Simple<char>> {
    just("//")
        .then(none_of('/'))
        .then(take_to_newline_or_end())
        .to(NatSpecBuilder::CommentedOutLine)
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

    let doc = choice([slashed_documentation, starred_documentation])
        .map_with_span(|spanned_body, span| (spanned_body, span));

    doc.then(associated_element().or_not())
        .map(
            |((spanned_body, span), element_under_doc)| NatSpecBuilder::Documentation {
                span,
                spanned_body,
                element_under_doc,
            },
        )
        .boxed()
}

pub(super) fn parser() -> impl Parser<char, Vec<Spanned<NatSpecBuilder>>, Error = Simple<char>> {
    let valid_natspec = choice([free_form_comment(), natspec_doc()]);
    let natspec = choice((commented_out_block(), commented_out_line(), valid_natspec));

    natspec
        .recover_with(skip_until(['\n', ' '], |_| NatSpecBuilder::ParseError).consume_end())
        .map_with_span(|builder, span| (builder, span))
        .repeated()
        .boxed()
}

#[cfg(test)]
mod tests;
