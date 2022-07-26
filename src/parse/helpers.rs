use std::iter;

use super::*;
use chumsky::prelude::*;

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

pub const INVARIANT_STOP_TOKENS: [Token; 14] = [
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
    Token::Hook,
    Token::Axiom,
    Token::Using,
    Token::Hook,
];

pub(super) fn newline<'src>() -> impl Parser<char, &'src str, Error = Simple<char>> + Clone {
    static NEWLINE: &[&str; 2] = &["\r\n", "\n"];
    let newline_parsers = NEWLINE.map(just);
    choice(newline_parsers)
}

pub(super) fn newline_or_end<'src>() -> impl Parser<char, &'src str, Error = Simple<char>> + Clone {
    let end = end().to("");
    newline().or(end).boxed()
}

pub(super) fn balanced(
    l: Token,
    r: Token,
) -> impl Parser<Token, Vec<Token>, Error = Simple<Token>> + Clone {
    let open = just(l.clone());
    let close = just(r.clone());

    let content = none_of([l, r]).at_least_once().boxed();

    recursive(|block| {
        let between = content.or(block).repeated().flatten();

        open.chain(between).chain(close)
    })
}

pub(super) fn balanced_stringified(
    l: Token,
    r: Token,
) -> impl Parser<Token, String, Error = Simple<Token>> + Clone {
    balanced(l, r).map(String::from_iter)
}

pub(super) fn mapping_ty() -> impl Parser<Token, String, Error = Simple<Token>> + Clone {
    just(Token::Mapping)
        .ignore_then(balanced_stringified(Token::RoundOpen, Token::RoundClose))
        .map(|content| format!("mapping{content}"))
}

pub(super) fn ident() -> impl Parser<Token, String, Error = Simple<Token>> + Clone {
    select! { Token::Ident(ident) => ident }
}

pub(super) fn ty() -> impl Parser<Token, String, Error = Simple<Token>> + Clone {
    let call = ident()
        .then_ignore(just(Token::Dot))
        .then(ident())
        .map(|(lhs, rhs)| format!("{lhs}.{rhs}"));

    let array_ty = {
        let array_subscript =
            balanced_stringified(Token::SquareOpen, Token::SquareClose).at_least_once();
        let caller = ident().or(call.clone());

        caller
            .then(array_subscript)
            .map(|(caller, subscript)| iter::once(caller).chain(subscript).collect())
    };

    choice((array_ty, mapping_ty(), call, ident())).labelled("type")
}

pub(super) fn param_list(
) -> impl Parser<Token, Vec<(String, Option<String>)>, Error = Simple<Token>> + Clone {
    ty().then(ident().or_not())
        .separated_by(just(Token::Comma))
        .delimited_by(just(Token::RoundOpen), just(Token::RoundClose))
        .labelled("param list")
}

pub(super) fn code_block() -> impl Parser<Token, Span, Error = Simple<Token>> + Clone {
    balanced(Token::CurlyOpen, Token::CurlyClose).map_with_span(|_, span| span)
}

pub(super) fn returns_type() -> impl Parser<Token, String, Error = Simple<Token>> + Clone {
    just(Token::Returns).ignore_then(ty())
}

pub(super) trait ParserExt<I, O, P> {
    fn at_least_once(self) -> chumsky::combinator::Repeated<P>;
}

impl<I: Clone, O, P: Parser<I, O>> ParserExt<I, O, P> for P {
    fn at_least_once(self) -> chumsky::combinator::Repeated<P> {
        self.repeated().at_least(1)
    }
}
