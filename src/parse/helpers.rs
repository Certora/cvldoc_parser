use chumsky::combinator::Repeated;
use chumsky::prelude::*;
use chumsky::primitive::OneOf;
use std::hash::Hash;

pub(super) fn newline<'src>() -> impl Parser<char, &'src str, Error = Simple<char>> + Clone {
    const NEWLINE: &[&str; 2] = &["\r\n", "\n"];
    let newline_parsers = NEWLINE.map(just);
    choice(newline_parsers)
}

pub(super) fn newline_or_end<'src>() -> impl Parser<char, &'src str, Error = Simple<char>> + Clone {
    let end = end().to("");
    newline().or(end).boxed()
}

pub(super) fn horizontal_ws<'src>() -> Repeated<OneOf<char, &'src [char; 2], Simple<char>>> {
    const HORIZONTAL_WHITESPACE: &[char; 2] = &[' ', '\t'];
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
    take_until_without_terminator(newline_or_end()).boxed()
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
    //we want to avoid parsing "/**" as a cvl comment, to give priority to starred natspec comments.
    //however, this creates an edge case.
    let edge_case_starter = just("/**/");
    let multi_line_starter = just("/*").then_ignore(none_of('*'));

    choice((edge_case_starter, multi_line_starter))
        .rewind()
        .then(take_to_starred_terminator())
        .ignored()
}

//grabs all text between a pair of curly brackets, including the brackets.
//it keeps going through nested brackets, until it find a closing bracket that
//matches the opening curly bracket (that is, the string up to that point is "balanced")
//note this does not validate that the brackets are
//still balanced past the last balanced closing bracket.
pub(super) fn balanced_brackets<'src>() -> Recursive<'src, char, String, Simple<char>> {
    let lb = just('{').map(String::from);
    let rb = just('}').map(String::from);
    let content = none_of("{}").repeated().at_least(1).map(String::from_iter);

    recursive(|block| {
        let between = content.or(block).repeated().map(String::from_iter);

        lb.chain(between)
            .chain(rb)
            .map(|v: Vec<String>| v.into_iter().collect())
    })
}
