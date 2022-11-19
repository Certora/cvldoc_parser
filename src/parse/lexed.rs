use std::ops::Not;

use itertools::Itertools;

use crate::{parse::terminated_line::TerminatedString, util::span_to_range::Span};

use super::{terminated_line::LinesWithEndings, *};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Token {
    Ghost,
    Definition,
    Rule,
    Invariant,
    Methods,
    Function,
    Mapping,
    Returns,
    Filtered,
    CvlDocSlashed(String),
    CvlDocStarred(String),
    FreeFormSlashed(String),
    FreeFormStarred(String),
    Bracket(char),
    Other(String),
    SingleLineComment,
    MultiLineComment,
}

pub fn lexer() -> impl Parser<char, Vec<(Token, Span)>, Error = Simple<char>> {
    let concat = |(prefix, (tail, terminator)): (&str, (Vec<char>, &str))| {
        prefix
            .chars()
            .chain(tail)
            .chain(terminator.chars())
            .collect()
    };
    let cvldoc_slashed_line = just("///")
        .then_ignore(none_of('/').rewind())
        .then(take_until(newline_or_end()))
        .map(concat);
    let cvldoc_slashed = cvldoc_slashed_line
        .repeated()
        .at_least(1)
        .map(|lines| lines.into_iter().collect())
        .map(Token::CvlDocSlashed)
        .boxed();
    let cvldoc_starred = just("/**")
        .then_ignore(none_of('*'))
        .then(take_until(just("*/")))
        .map(concat)
        .map(Token::CvlDocStarred)
        .boxed();
    let freeform_slashed_line = just("////").then(take_until(newline_or_end())).map(concat);
    let freeform_slashed = freeform_slashed_line
        .repeated()
        .at_least(1)
        .map(|lines| lines.into_iter().collect())
        .map(Token::FreeFormSlashed)
        .boxed();
    let freeform_starred = just("/***")
        .then(take_until(just("*/")))
        .map(concat)
        .map(Token::FreeFormStarred)
        .boxed();
    let freeform_starred_alternative = {
        //this is verbose and hideous

        let middle_endings = choice((just("*/\r\n"), just("*/\n")));
        let header_endings = choice((just("/\r\n"), just("/\n"), just("/").then_ignore(end())));

        let header = just("/***")
            .then(just('*').repeated())
            .then(header_endings.clone())
            .map(|((header, content), terminator)| {
                header
                    .chars()
                    .chain(content.into_iter())
                    .chain(terminator.chars())
                    .collect::<String>()
            })
            .boxed();
        let middle = just("/***")
            .then(take_until(middle_endings))
            .map(
                |(header, (content, terminator)): (&str, (Vec<char>, &str))| {
                    header
                        .chars()
                        .chain(content.into_iter())
                        .chain(terminator.chars())
                        .collect::<String>()
                },
            )
            .boxed();
        header
            .clone()
            .then(middle)
            .then(header)
            .map(|((top, middle), bottom)| {
                top.chars()
                    .chain(middle.chars())
                    .chain(bottom.chars())
                    .collect()
            })
            .map(Token::FreeFormStarred)
            .boxed()
    };

    let bracket = one_of("()[]{}").map(Token::Bracket).boxed();

    let single_line_comment = just("//")
        .then(none_of('/'))
        .then(take_until(newline_or_end()))
        .to(Token::SingleLineComment);
    let multi_line_comment = just("/*")
        .then(none_of('*'))
        .then(take_until(just("*/")))
        .to(Token::MultiLineComment);
    let comment = single_line_comment.or(multi_line_comment).boxed();

    let keyword_or_other = text::ident()
        .map(|ident: String| match ident.as_str() {
            "ghost" => Token::Ghost,
            "definition" => Token::Definition,
            "rule" => Token::Rule,
            "invariant" => Token::Invariant,
            "methods" => Token::Methods,
            "function" => Token::Function,
            "mapping" => Token::Mapping,
            "returns" => Token::Returns,
            "filtered" => Token::Filtered,
            _ => Token::Other(ident),
        })
        .boxed();
    let not_whitespace = |c: &char| c.is_ascii_whitespace().not();
    let other = filter(not_whitespace)
        .repeated()
        .at_least(1)
        .collect()
        .map(Token::Other)
        .boxed();

    choice([
        cvldoc_slashed,
        cvldoc_starred,
        freeform_slashed,
        freeform_starred_alternative,
        freeform_starred,
        comment,
        bracket,
        keyword_or_other,
        other,
    ])
    .map_with_span(|token, span| (token, span))
    .padded()
    .repeated()
}
fn add_span<T>(token: T, span: Span) -> (T, Span) {
    (token, span)
}

#[derive(Debug)]
pub enum Intermediate {
    FreeForm(String),
    CvlDoc(String),
}

pub fn intermediate_parser() -> impl Parser<Token, Vec<(Intermediate, Span)>, Error = Simple<Token>>
{
    let freeform_slashed = select! {
        Token::FreeFormSlashed(lines) => Intermediate::FreeForm(lines),
        Token::FreeFormStarred(lines) => Intermediate::FreeForm(lines),
    };

    let cvldoc = select! {
        Token::CvlDocSlashed(lines) => Intermediate::CvlDoc(lines),
        Token::CvlDocStarred(lines) => Intermediate::CvlDoc(lines),
    };

    let comment = choice([
        just(Token::SingleLineComment),
        just(Token::MultiLineComment),
    ]);
    choice((cvldoc, freeform_slashed))
        .map_with_span(add_span)
        .padded_by(comment.repeated())
        .repeated()
}

fn cleanup(lines: Vec<(String, Span)>) -> Vec<(TerminatedString, Span)> {
    static PADDING: &[char] = &[' ', '\t', '/', '*'];
    lines
        .into_iter()
        .map(|(line, span)| {
            let line = line.trim_end_matches(PADDING);
            let terminated = TerminatedString::from_str(line);

            (terminated, span)
        })
        .collect()
}

// pub fn actual_parser() -> impl Parser<Token, Vec<CvlDocBuilder>, Error = Simple<Token>> {
//     let freeform = select! {
//         Token::FreeFormSlashed(text) => text,
//         Token::FreeFormStarred(text) => text,
//     }
//     .map_with_span(|text, span| CvlDocBuilder::FreeFormComment { text, span });

//     let cvldoc_slashed =
//         select! { Token::CvlDocSlashed(text) => text }.map_with_span(|text, span| {
//             let lines = LinesWithEndings::from(&text, span);
//             lines
//                 .into_iter()
//                 .map(|(line, span)| {
//                     let line = line.trim_start_matches('/').trim_start();
//                     (line.to_string(), span)
//                 })
//                 .collect_vec()
//         });

//     let cvldoc_starred =
//         select! { Token::CvlDocStarred(text) => text }.map_with_span(|text, span| {
//             let lines = LinesWithEndings::from(&text, span);
//             lines
//                 .into_iter()
//                 .map(|(line, span)| {
//                     static PADDING: &[char] = &[' ', '/', '*'];
//                     let line = line.trim_start_matches(PADDING);
//                     (line.to_string(), span)
//                 })
//                 .collect_vec()
//         });

//     let comment = choice([
//         just(Token::SingleLineComment),
//         just(Token::MultiLineComment),
//     ]);

//     choice((cvldoc_slashed, freeform))
//         .map_with_span(add_span)
//         .padded_by(comment.repeated())
//         .repeated()
// }
