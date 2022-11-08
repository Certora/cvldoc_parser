mod associated_element;
pub mod builder;
mod helpers;
mod terminated_line;

use self::associated_element::associated_element;
use crate::parse::terminated_line::JoinToString;
use builder::CvlDocBuilder;
use chumsky::{prelude::*, text::whitespace};
use helpers::*;
use std::iter;
use terminated_line::TerminatedLine;

fn free_form_comment<'src>() -> BoxedParser<'src, char, CvlDocBuilder, Simple<char>> {
    let slashes = just('/').repeated().at_least(4);
    let thick_slashed_padding = slashes.then(newline_or_end());

    let slashed_free_form_line = slashes
        .ignore_then(horizontal_ws())
        .ignore_then(line_with_terminator())
        .map(|line| line.trim_end(&[' ', '\t', '/']))
        .boxed();

    let slashed_free_form = slashed_free_form_line
        .clone()
        .repeated()
        .at_least(1)
        .map(JoinToString::join_to_string)
        .boxed();
    let slashed_thick_free_form = slashed_free_form_line
        .padded_by(thick_slashed_padding)
        .map(|line| iter::once(line).join_to_string())
        .boxed();

    let stars = just('*').repeated().at_least(3);
    let thick_starred_padding = just('/').then(stars).then(just('/')).then(newline_or_end());

    let starred_header = {
        let endings = choice((just("*/\r\n"), just("*/\n"), just("*/").then_ignore(end())));
        let content = take_until_without_terminator(endings);

        just("/***")
            .ignore_then(content)
            .map(|line| {
                static PADDING: &[char] = &[' ', '\t', '*'];
                let line = String::from_iter(line);
                line.trim_matches(PADDING).to_string()
            })
            .boxed()
    };

    let starred_single_line_free_form = starred_header.clone();
    let starred_thick_free_form = starred_header.padded_by(thick_starred_padding).boxed();

    let starred_multi_line_first_line = just("/***").then(newline()).boxed();
    let starred_body = take_to_starred_terminator()
        .then_ignore(newline_or_end())
        .map(|body| {
            static PADDING: &[char] = &[' ', '\t', '*'];
            body.split_inclusive(|&c| c == '\n')
                .map(TerminatedLine::from_char_slice)
                .map(|term_line| term_line.trim_start(PADDING).trim_end(PADDING))
                .join_to_string()
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
    .map_with_span(|text, span| CvlDocBuilder::FreeFormComment { text, span })
    .boxed()
}

fn commented_out_block<'src>() -> BoxedParser<'src, char, CvlDocBuilder, Simple<char>> {
    multi_line_cvl_comment()
        .to(CvlDocBuilder::CommentedOutBlock)
        .boxed()
}

fn commented_out_line<'src>() -> BoxedParser<'src, char, CvlDocBuilder, Simple<char>> {
    just("//")
        .then(none_of('/'))
        .then(take_to_newline_or_end())
        .to(CvlDocBuilder::CommentedOutLine)
        .boxed()
}

fn cvldoc_documentation<'src>() -> BoxedParser<'src, char, CvlDocBuilder, Simple<char>> {
    let spanned_slashed_line = just("///")
        .ignore_then(none_of('/').rewind())
        .ignore_then(horizontal_ws())
        .ignore_then(line_with_terminator().map_with_span(|line, span| {
            static PADDING: &[char] = &[' ', '\t'];
            (line.trim_end(PADDING), span)
        }))
        .boxed();

    let slashed_documentation = spanned_slashed_line.repeated().at_least(1).boxed();

    let starred_documentation = just("/**")
        .then(none_of("*/").rewind())
        .then_ignore(whitespace())
        .ignore_then(take_to_starred_terminator().map_with_span(builder::split_starred_doc_lines))
        .boxed();

    let documentation = choice([slashed_documentation, starred_documentation])
        .map_with_span(|spanned_body, span| (spanned_body, span));

    documentation
        .then(associated_element().or_not())
        .map(
            |((spanned_body, span), associated)| CvlDocBuilder::Documentation {
                span,
                spanned_body,
                associated,
            },
        )
        .boxed()
}

pub(super) fn parser() -> impl Parser<char, Vec<CvlDocBuilder>, Error = Simple<char>> {
    let valid_cvldoc = choice([free_form_comment(), cvldoc_documentation()]);
    let cvldoc = choice((commented_out_block(), commented_out_line(), valid_cvldoc));

    cvldoc
        .recover_with(skip_until(['\n', ' '], |_| CvlDocBuilder::ParseError).consume_end())
        .repeated()
        .boxed()
}

#[cfg(test)]
mod tests;
