mod associated_element;
pub mod builder;
mod helpers;

use self::associated_element::associated_element;
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
    .map_with_span(|header, span| NatSpecBuilder::FreeFormComment {
        header,
        block: None,
        span,
    });

    let multi_line_first_line = just('/').then(stars).then(newline()).boxed();
    let multi_line_body = take_until(just('#'))
        .ignore_then(take_to_starred_terminator())
        .then_ignore(newline_or_end())
        .collect()
        .boxed();
    let free_form_multi_line = multi_line_first_line
        .ignore_then(multi_line_body)
        .map_with_span(NatSpecBuilder::free_form_multi_line_from_body_and_span)
        .boxed();

    choice((free_form_single_line, free_form_multi_line)).boxed()
}

fn commented_out_block<'src>() -> BoxedParser<'src, char, NatSpecBuilder, Simple<char>> {
    multi_line_cvl_comment()
        .to(NatSpecBuilder::CommentedOutBlock)
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
    let natspec = commented_out_block().or(valid_natspec);

    natspec
        .recover_with(skip_until(['\n', ' '], |_| NatSpecBuilder::ParseError).consume_end())
        .map_with_span(|builder, span| (builder, span))
        .repeated()
        .boxed()
}

#[cfg(test)]
mod tests;
