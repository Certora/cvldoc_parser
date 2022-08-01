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
    .map_with_span(|header, span| NatSpecBuilder::FreeFormComment {
        header,
        block: None,
        span,
    });

    let multi_line_first_line = just('/').then(stars).then(just('\n')).boxed();
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

/// when parsing the block associated with the documentation, we are dealing with
/// a stream of tokens. tokens may be separated by some combination of whitespace or comments.
/// since we do not go through a lexing stage that filters them out, we must assume
/// that they may exist (possibly repeatedly) between any valid token of the associated block.
fn optional_token_separator_immediately_after_doc<'src>(
) -> BoxedParser<'src, char, (), Simple<char>> {
    let single_line_comment_between_tokens = just("//")
        .then(none_of('/').rewind())
        .then(take_to_newline_or_end())
        .ignored();

    //we cannot use the usual multi-line comment parser here, since it is
    //now allowed to have "/**" as a comment starter.
    let multi_line_comment_between_tokens = just("/*").then(take_to_starred_terminator()).ignored();

    let comment = choice((
        single_line_comment_between_tokens,
        multi_line_comment_between_tokens,
    ))
    .padded();

    comment.repeated().ignored().padded().boxed()
}

fn optional_token_separator<'src>() -> BoxedParser<'src, char, (), Simple<char>> {
    //we cannot use the usual multi-line comment parser here, since it is
    //now allowed to have "/**" as a comment starter.
    let multi_line_comment_between_tokens = just("/*").then(take_to_starred_terminator()).ignored();

    let comment = choice((single_line_cvl_comment(), multi_line_comment_between_tokens)).padded();

    comment.repeated().ignored().padded().boxed()
}

/// according to the grammar, it is required to have some amount of whitespace immediately after
/// some tokens. however, this may again be followed by comments.
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

        optional_token_separator_immediately_after_doc()
            .ignore_then(decl)
            .then_ignore(optional_token_separator())
            .boxed()
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

    //grabs the code block associated with the function,
    //including any delimiting brackets.
    //it checks for balanced brackets inside the block, starting
    //from an opening curly bracket, and keeps going until it
    //detects a closing bracket such that the block from the start
    //is balanced. note this does not validate that the brackets are
    //still balanced past the last balanced closing bracket.
    let block_under_natspec = {
        let lb = just('{').map(String::from);
        let rb = just('}').map(String::from);
        let content = none_of("{}").repeated().at_least(1).map(String::from_iter);

        recursive(|block| {
            let between = content.or(block).repeated().map(String::from_iter);

            lb.chain(between)
                .chain(rb)
                .map(|v: Vec<String>| v.into_iter().collect())
        })
    };

    decl_under_natspec
        .then(params_under_natspec.or_not().map(Option::unwrap_or_default))
        .then_ignore(optional_token_separator())
        .then(block_under_natspec)
        .map(|(((kind, name), params), block)| UnderDoc {
            kind,
            name,
            params,
            block,
        })
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

    doc.then(under_doc().or_not())
        .map(
            |((spanned_body, span), element_under_doc)| NatSpecBuilder::Documentation {
                span,
                spanned_body,
                element_under_doc,
            },
        )
        .boxed()
}

fn commented_out_block<'src>() -> BoxedParser<'src, char, NatSpecBuilder, Simple<char>> {
    multi_line_cvl_comment()
        .to(NatSpecBuilder::CommentedOutBlock)
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
