pub mod slot;

use super::*;
use crate::Param;
use chumsky::prelude::*;
use itertools::Itertools;
use std::iter;

pub const SYNC_TOKENS: [Token; 10] = [
    Token::FreeFormSlashed,
    Token::FreeFormStarred,
    Token::CvlDocSlashed,
    Token::CvlDocStarred,
    Token::Ghost,
    Token::Definition,
    Token::Rule,
    Token::Invariant,
    Token::Methods,
    Token::Function,
];

pub(super) fn newline<'src>() -> impl Parser<char, &'src str, Error = Simple<char>> {
    static NEWLINE: &[&str; 2] = &["\r\n", "\n"];
    let newline_parsers = NEWLINE.map(just);
    choice(newline_parsers)
}

pub(super) fn newline_or_end<'src>() -> impl Parser<char, &'src str, Error = Simple<char>> {
    let end = end().to("");
    newline().or(end)
}

pub(super) fn balanced(
    l: Token,
    r: Token,
) -> impl Parser<Token, Vec<Token>, Error = Simple<Token>> {
    let open = just(l.clone());
    let close = just(r.clone());

    let content = none_of([l, r]).repeated().at_least(1);

    recursive(|block| {
        let between = content.or(block).repeated().flatten();

        open.chain(between).chain(close)
    })
}

pub(super) fn balanced_stringified(
    l: Token,
    r: Token,
) -> impl Parser<Token, String, Error = Simple<Token>> {
    balanced(l, r).map(String::from_iter)
}

pub(super) fn mapping_ty() -> impl Parser<Token, String, Error = Simple<Token>> {
    just(Token::Mapping)
        .ignore_then(balanced_stringified(Token::RoundOpen, Token::RoundClose))
        .map(|content| format!("mapping{content}"))
}

pub(super) fn ident() -> impl Parser<Token, String, Error = Simple<Token>> {
    select! { Token::Ident(ident) => ident }
}

pub(super) fn string() -> impl Parser<Token, String, Error = Simple<Token>> {
    select! { Token::String(s) => s }
}

pub(super) fn function_ident() -> impl Parser<Token, String, Error = Simple<Token>> {
    select! { Token::Ident(ident) => ident }
        .separated_by(just(Token::Dot))
        .map(|sections| sections.into_iter().join("."))
}

pub(super) fn ty() -> impl Parser<Token, String, Error = Simple<Token>> {
    let array_ty = {
        let array_subscript = balanced_stringified(Token::SquareOpen, Token::SquareClose)
            .repeated()
            .at_least(1);
        let caller = ident().or(call());

        caller
            .then(array_subscript)
            .map(|(caller, subscript)| iter::once(caller).chain(subscript).collect())
    };

    choice((array_ty, mapping_ty(), call(), ident())).labelled("type")
}

pub(super) fn call() -> impl Parser<Token, String, Error = Simple<Token>> {
    ident()
        .then_ignore(just(Token::Dot))
        .then(ident())
        .map(|(lhs, rhs)| format!("{lhs}.{rhs}"))
}

pub(super) fn single_expr() -> impl Parser<Token, (), Error = Simple<Token>> {
    // this is a massive over-approximation of an expression,
    // but (assuming correct code) it's good enough for invariants as of CVL2
    let expression_enders = [Token::Semicolon, Token::CurlyOpen];

    none_of(expression_enders).repeated().at_least(1).ignored()
}

pub(super) fn unnamed_param_list() -> impl Parser<Token, Vec<String>, Error = Simple<Token>> {
    ty().separated_by(just(Token::Comma))
        .delimited_by(just(Token::RoundOpen), just(Token::RoundClose))
        .labelled("unnamed param list")
}

pub(super) fn named_param_list() -> impl Parser<Token, Vec<Param>, Error = Simple<Token>> {
    named_param()
        .separated_by(just(Token::Comma))
        .delimited_by(just(Token::RoundOpen), just(Token::RoundClose))
        .labelled("named param list")
}

pub(super) fn named_param() -> impl Parser<Token, Param, Error = Simple<Token>> {
    ty().then(ident()).map(|(ty, name)| Param::new(ty, name))
}

pub(super) fn num() -> impl Parser<Token, String, Error = Simple<Token>> {
    select! { Token::Number(n) => n }
}

pub(super) fn code_block() -> impl Parser<Token, Span, Error = Simple<Token>> {
    balanced(Token::CurlyOpen, Token::CurlyClose).map_with_span(|_, span| span)
}

pub(super) fn optional_code_block() -> impl Parser<Token, Option<Span>, Error = Simple<Token>> {
    choice((code_block().map(Some), semicolon_ender()))
}

pub(super) fn filtered_block() -> impl Parser<Token, Span, Error = Simple<Token>> {
    just(Token::Filtered).ignore_then(code_block())
}

pub(super) fn semicolon_ender() -> impl Parser<Token, Option<Span>, Error = Simple<Token>> {
    just(Token::Semicolon).to(None)
}

pub(super) fn returns_type() -> impl Parser<Token, String, Error = Simple<Token>> {
    just(Token::Returns).ignore_then(ty())
}
