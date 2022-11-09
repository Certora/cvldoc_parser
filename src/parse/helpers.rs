use super::terminated_line::{TerminatedLine, Terminator};
use chumsky::combinator::Repeated;
use chumsky::prelude::*;
use chumsky::primitive::OneOf;
use std::hash::Hash;

pub(super) fn newline<'src>() -> impl Parser<char, &'src str, Error = Simple<char>> + Clone {
    static NEWLINE: &[&str; 2] = &["\r\n", "\n"];
    let newline_parsers = NEWLINE.map(just);
    choice(newline_parsers)
}

pub(super) fn newline_or_end<'src>() -> impl Parser<char, &'src str, Error = Simple<char>> + Clone {
    let end = end().to("");
    newline().or(end).boxed()
}

pub(super) fn line_with_terminator(
) -> impl Parser<char, TerminatedLine, Error = Simple<char>> + Clone {
    let terminator = choice([
        just("\r\n").to(Terminator::CRLF).boxed(),
        just('\n').to(Terminator::LF).boxed(),
        just('\r').to(Terminator::CR).boxed(),
        end().to(Terminator::EOF).boxed(),
    ]);

    take_until(terminator).map(|(content, terminator)| TerminatedLine::new(content, terminator))
}

pub(super) fn horizontal_ws<'src>() -> Repeated<OneOf<char, &'src [char; 2], Simple<char>>> {
    static HORIZONTAL_WHITESPACE: &[char; 2] = &[' ', '\t'];
    one_of(HORIZONTAL_WHITESPACE).repeated()
}

/// a version of `take_until` that only collects the input before the terminator,
/// and drops the output of the terminating pattern parser
pub(super) fn take_until_without_terminator<I, O>(
    terminator: impl Parser<I, O, Error = Simple<I>> + Clone,
) -> impl Parser<I, Vec<I>, Error = Simple<I>> + Clone
where
    I: Clone + Hash + Eq,
{
    let ignore_terminator = |(a, _b)| a;
    take_until(terminator).map(ignore_terminator)
}

pub(super) fn take_to_newline_or_end<'src>() -> BoxedParser<'src, char, Vec<char>, Simple<char>> {
    take_until(newline_or_end())
        .map(|(mut content, line_end)| {
            content.extend(line_end.chars());
            content
        })
        .boxed()
}

pub(super) fn take_to_starred_terminator<'src>() -> BoxedParser<'src, char, Vec<char>, Simple<char>>
{
    take_until_without_terminator(just("*/")).boxed()
}

pub(super) fn single_line_cvl_comment() -> impl Parser<char, (), Error = Simple<char>> {
    just("//").then(take_to_newline_or_end()).ignored()
}

pub(super) fn multi_line_cvl_comment() -> impl Parser<char, (), Error = Simple<char>> {
    //this is a somewhat tricky parse.
    //we want to avoid parsing "/**" as a cvl comment, to give priority to starred cvldoc comments.
    //however, this creates an edge case.
    let edge_case_starter = just("/**/");
    let multi_line_starter = just("/*").then_ignore(none_of('*'));

    choice((edge_case_starter, multi_line_starter))
        .rewind()
        .then(take_to_starred_terminator())
        .ignored()
}


/// when parsing the block associated with the documentation, we are dealing with
/// a stream of tokens. tokens may be separated by some combination of whitespace or comments.
/// since we do not go through a lexing stage that filters them out, we must assume
/// that they may exist (possibly repeatedly) between any valid token of the associated block.
pub(super) fn optional_sep_immediately_after_doc<'src>() -> BoxedParser<'src, char, (), Simple<char>> {
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
